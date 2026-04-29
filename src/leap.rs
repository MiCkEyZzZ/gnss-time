//! # Leap seconds — conversion context
//!
//! ## Why this is an explicit parameter, not global state
//!
//! ```text
//! // ❌ Hidden state — bad
//! let utc = gps.to_utc(); // where do the leap seconds come from?
//!
//! // ✅ Explicit context — good
//! let utc = gps_to_utc(gps, LeapSeconds::builtin())?;
//! ```
//!
//! Reasons:
//! - `no_std` / embedded: there is no global mutable memory
//! - Embedded GNSS receiver: the table is read from the almanac and updated at
//!   runtime
//! - Testing: easy to inject the desired state without mocks
//! - Determinism: compiled code does not depend on future IERS updates
//!
//! ## Supported conversions
//!
//! | Function            | Leap-second context?        |
//! |--------------------|-----------------------------|
//! | `glonass_to_utc`   | **no** (constant shift)     |
//! | `utc_to_glonass`   | **no** (constant shift)     |
//! | `gps_to_utc`       | yes                         |
//! | `utc_to_gps`       | yes                         |
//! | `gps_to_glonass`   | yes (via UTC)               |
//! | `glonass_to_gps`   | yes (via UTC)               |
//!
//! ## GLONASS and leap seconds
//!
//! GLONASS tracks UTC(SU) = UTC + 3 hours, including leap-second insertions.
//! Therefore GLONASS ↔ UTC conversion is a **constant shift** in nanoseconds
//! (the difference between epochs), without any leap-second adjustments.
//! Leap seconds are only needed when crossing into GPS/Galileo/BeiDou.

use crate::{
    tables::BUILTIN_TABLE, Beidou, CivilDate, Galileo, Glonass, GnssTimeError, Gps, Tai, Time, Utc,
};

static BUILTIN_LEAP_SECONDS: LeapSeconds = LeapSeconds {
    entries: &BUILTIN_TABLE,
};

/// Nanoseconds from the UTC epoch (1972-01-01) to the GLONASS epoch
/// (1995-12-31 21:00:00 UTC).
///
/// GLONASS epoch = 1996-01-01 00:00:00 UTC(SU) = 1995-12-31 21:00:00 UTC.
///
/// `UTC_nanos = GLO_nanos + GLONASS_FROM_UTC_EPOCH_NS`
const GLONASS_FROM_UTC_EPOCH_NS: i64 = {
    // от UTC-epoch до 1996-01-01 00:00:00 UTC
    let to_1996 = CivilDate::new(1972, 1, 1).nanos_until(CivilDate::new(1996, 1, 1));

    // minus 3 hours: GLONASS epoch = 3 hours earlier in UTC
    to_1996 - 3 * 3_600 * 1_000_000_000_i64
    // = 8766 days * 86400 * 1e9 - 10800 * 1e9
    // = 757_382_400_000_000_000 - 10_800_000_000_000 = 757_371_600_000_000_000
};

const _VERIFY_GLONASS_OFFSET: () = {
    let s = GLONASS_FROM_UTC_EPOCH_NS / 1_000_000_000;

    assert!(
        s == 757_371_600,
        "GLONASS -> UTC epoch offset must be 757371600 s"
    );
};

/// Nanoseconds from the UTC epoch (1972-01-01) to the GPS epoch (1980-01-06).
///
/// The GPS epoch is later, so the value is positive.
/// `UTC_nanos_from_1972 = GPS_nanos_from_1980 - (TAI_minus_UTC - 19) * 1e9 +
/// THIS`
const UTC_TO_GPS_EPOCH_NS: i64 = CivilDate::new(1972, 1, 1).nanos_until(CivilDate::new(1980, 1, 6));
// = 2927 days * 86400 * 1e9 = 252_892_800_000_000_000 ns

const _VERIFY_UTC_GPS_OFFSET: () = {
    let s = UTC_TO_GPS_EPOCH_NS / 1_000_000_000;

    assert!(
        s == 252_892_800,
        "UTC -> GPS epoch offset must be 252892800 s (2927 days)"
    );
};

/// Source of TAI-UTC corrections for conversions involving UTC and GLONASS.
///
/// This makes it possible to provide custom tables, for example values read
/// from a GNSS receiver almanac, without changing the crate code.
///
/// # Example
///
/// ```rust
/// use gnss_time::{LeapEntry, LeapSecondsProvider, Tai, Time};
///
/// struct FixedLeap(i32);
///
/// impl LeapSecondsProvider for FixedLeap {
///     fn tai_minus_utc_at(
///         &self,
///         _tai: Time<Tai>,
///     ) -> i32 {
///         self.0
///     }
/// }
/// ```
pub trait LeapSecondsProvider {
    /// Returns TAI - UTC (in seconds) for the given TAI moment.
    fn tai_minus_utc_at(
        &self,
        tai: Time<Tai>,
    ) -> i32;
}

/// One leap-second table entry.
///
/// Starting from `tai_minus_utc` (internal TAI nanoseconds), `TAI - UTC =
/// tai_minus_utc` seconds.
///
/// Strict contract: the table must be sorted by `tai_nanos` in ascending order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeapEntry {
    /// Internal TAI nanoseconds (inclusive lower bound).
    pub tai_nanos: u64,

    /// TAI - UTC in whole seconds, valid from this moment onward.
    pub tai_minus_utc: i32,
}

/// Static leap-second correction table.
///
/// The built-in table [`builtin`](LeapSeconds::builtin) covers all events from
/// the GPS start (1980-01-06) through 2017-01-01 inclusive.
/// For times after the last entry, the last known value is returned
/// (the standard "assume no new leap seconds" approach).
///
/// # no_std
///
/// `LeapSeconds` stores `&'static [LeapEntry]` — there are no allocations, and
/// it works everywhere.
///
/// # Examples
///
/// ```rust
/// use gnss_time::{gps_to_utc, Gps, LeapSeconds, LeapSecondsProvider, Time};
///
/// // Built-in table (up to 2017)
/// let ls = LeapSeconds::builtin();
///
/// let gps = Time::<Gps>::from_week_tow(1981, 0.0).unwrap();
/// let utc = gps_to_utc(gps, &ls).unwrap();
/// // GPS leads UTC by 18 seconds in this period
/// ```
pub struct LeapSeconds {
    entries: &'static [LeapEntry], // (Unix seconds, TAI-UTC)
}

impl LeapEntry {
    /// Creates a new leap-second entry.
    ///
    /// # Parameters
    /// - `tai_nanos`: threshold value in TAI nanoseconds (inclusive lower
    ///   bound) from which this offset applies.
    /// - `tai_minus_utc`: TAI - UTC in seconds that applies from this
    ///   threshold.
    #[inline]
    pub const fn new(
        tai_nanos: u64,
        tai_minus_utc: i32,
    ) -> Self {
        LeapEntry {
            tai_nanos,
            tai_minus_utc,
        }
    }
}

impl LeapSeconds {
    /// Built-in table valid through 2017-01-01.
    ///
    /// Covers all 18 leap-second events in the GPS era.
    ///
    /// Source: [IERS Bulletin C](https://www.iers.org/IERS/EN/Publications/Bulletins/bulletins.html)
    pub fn builtin() -> &'static LeapSeconds {
        &BUILTIN_LEAP_SECONDS
    }

    /// Creates a table from a custom slice (for example, loaded from a
    /// receiver).
    ///
    /// # Requirements
    ///
    /// `entries` must be sorted by `tai_nanos` in ascending order.
    pub const fn from_table(entries: &'static [LeapEntry]) -> Self {
        Self { entries }
    }

    /// Returns the number of entries in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns all table entries (for inspection / serialization).
    pub fn entries(&self) -> &[LeapEntry] {
        self.entries
    }
}

impl LeapSecondsProvider for LeapSeconds {
    fn tai_minus_utc_at(
        &self,
        tai: Time<Tai>,
    ) -> i32 {
        let nanos = tai.as_nanos();
        let entries = self.entries;

        if entries.is_empty() {
            return 19; // safe fallback value at GPS epoch
        }

        // Find the last entry with tai_nanos <= nanos
        match entries.binary_search_by_key(&nanos, |e| e.tai_nanos) {
            // Exact match: use found entry
            Ok(i) => entries[i].tai_minus_utc,
            // nanos is before first entry: return initial value
            Err(0) => entries[0].tai_minus_utc,
            // Standard case: entry before insertion point
            Err(i) => entries[i - 1].tai_minus_utc,
        }
    }
}

// Generic implementation: &P automatically implements LeapSecondsProvider if P
// does. This allows passing &LeapSeconds::builtin() directly.
impl<P: LeapSecondsProvider> LeapSecondsProvider for &P {
    fn tai_minus_utc_at(
        &self,
        tai: Time<Tai>,
    ) -> i32 {
        (*self).tai_minus_utc_at(tai)
    }
}

////////////////////////////////////////////////////////////////////////////////
// GLONASS -> UTC, GPS
////////////////////////////////////////////////////////////////////////////////

/// Converts GLONASS -> UTC (without leap-second context).
///
/// GLONASS tracks UTC(SU) = UTC + 3h, including leap seconds.
/// Both scales store continuous nanoseconds, so the conversion is just a
/// constant epoch shift.
///
/// # Shift
///
/// `UTC_ns = GLO_ns + 757_371_600_000_000_000`
/// (= days from UTC epoch to GLONASS epoch × 86400 × 1e9)
///
/// # Errors
///
/// [`GnssTimeError::Overflow`] — if UTC < UTC epoch (1972-01-01).
pub fn glonass_to_utc(glo: Time<Glonass>) -> Result<Time<Utc>, GnssTimeError> {
    let utc_ns = (glo.as_nanos() as i128) + (GLONASS_FROM_UTC_EPOCH_NS as i128);

    if utc_ns < 0 || utc_ns > u64::MAX as i128 {
        return Err(GnssTimeError::Overflow);
    }

    Ok(Time::<Utc>::from_nanos(utc_ns as u64))
}

/// Converts GLONASS -> GPS via UTC.
///
/// Requires leap-second context (for UTC -> GPS).
pub fn glonass_to_gps<P: LeapSecondsProvider>(
    glo: Time<Glonass>,
    ls: &P,
) -> Result<Time<Gps>, GnssTimeError> {
    let utc = glonass_to_utc(glo)?;

    utc_to_gps(utc, ls)
}

/// Converts GLONASS -> Galileo via UTC (requires leap-second context).
pub fn glonass_to_galileo<P: LeapSecondsProvider>(
    glo: Time<Glonass>,
    ls: &P,
) -> Result<Time<Galileo>, GnssTimeError> {
    let utc = glonass_to_utc(glo)?;

    utc_to_galileo(utc, ls)
}

/// Converts GLONASS -> BeiDou via UTC (requires leap-second context).
pub fn glonass_to_beidou<P: LeapSecondsProvider>(
    glo: Time<Glonass>,
    ls: &P,
) -> Result<Time<Beidou>, GnssTimeError> {
    let utc = glonass_to_utc(glo)?;

    utc_to_beidou(utc, ls)
}

////////////////////////////////////////////////////////////////////////////////
// GPS -> UTC, GLONASS
////////////////////////////////////////////////////////////////////////////////

/// Converts GPS -> UTC.
///
/// Requires an explicit [`LeapSecondsProvider`] context.
///
/// # Formula
///
/// ```text
/// UTC_nanos_from_1972 = GPS_nanos_from_1980 - (TAI_minus_UTC - 19) * 1e9 + GPS_EPOCH_OFFSET_FROM_UTC_EPOCH_ns
/// ```
///
/// # Errors
///
/// [`GnssTimeError::Overflow`] — the result does not fit into `u64`.
///
/// # Example
///
/// ```rust
/// use gnss_time::{gps_to_utc, Gps, LeapSeconds, Time};
///
/// let ls = LeapSeconds::builtin();
/// let gps = Time::<Gps>::from_nanos(0); // GPS epoch
/// let utc = gps_to_utc(gps, &ls).unwrap();
///
/// // At the GPS epoch (1980-01-06), GPS-UTC = 0; UTC should represent the same instant
/// assert_eq!(utc.as_nanos(), 252_892_800_000_000_000); // from 1972-01-01
/// ```
pub fn gps_to_utc<P: LeapSecondsProvider>(
    gps: Time<Gps>,
    ls: &P,
) -> Result<Time<Utc>, GnssTimeError> {
    let tai = gps.to_tai()?;
    let n = ls.tai_minus_utc_at(tai);
    // UTC_ns = GPS_ns - (n - 19) * 1e9 + epoch_offset
    let utc_ns = (gps.as_nanos() as i128) - ((n - 19) as i128 * 1_000_000_000_i128)
        + (UTC_TO_GPS_EPOCH_NS as i128);

    if utc_ns < 0 || utc_ns > u64::MAX as i128 {
        return Err(GnssTimeError::Overflow);
    }

    Ok(Time::<Utc>::from_nanos(utc_ns as u64))
}

/// Converts GPS -> GLONASS via UTC.
///
/// Requires leap-second context (for GPS -> UTC).
pub fn gps_to_glonass<P: LeapSecondsProvider>(
    gps: Time<Gps>,
    ls: &P,
) -> Result<Time<Glonass>, GnssTimeError> {
    let utc = gps_to_utc(gps, ls)?;

    utc_to_glonass(utc)
}

////////////////////////////////////////////////////////////////////////////////
// Galileo -> UTC, GLONASS
////////////////////////////////////////////////////////////////////////////////

/// Galileo -> UTC (requires leap-second context).
///
/// Galileo and GPS have the same TAI offset (19 s), so `GAL -> UTC` is
/// equivalent to `GPS -> UTC` (same nanoseconds, same context).
pub fn galileo_to_utc<P: LeapSecondsProvider>(
    gal: Time<Galileo>,
    ls: &P,
) -> Result<Time<Utc>, GnssTimeError> {
    // Galileo and GPS share the same TAI offset, so we convert via GPS as an
    // intermediate step.
    let gps = gal.try_convert::<Gps>()?;

    gps_to_utc(gps, ls)
}

/// Galileo -> GLONASS via UTC (requires leap-second context).
pub fn galileo_to_glonass<P: LeapSecondsProvider>(
    gal: Time<Galileo>,
    ls: &P,
) -> Result<Time<Glonass>, GnssTimeError> {
    let utc = galileo_to_utc(gal, ls)?;

    utc_to_glonass(utc)
}

////////////////////////////////////////////////////////////////////////////////
// BeiDou -> UTC
////////////////////////////////////////////////////////////////////////////////

/// BeiDou -> UTC (requires leap-second context).
///
/// BDT = GPS − 14 s (via TAI: BDT + 33 s = TAI = GPS + 19 s).
/// `BDT -> UTC` is converted through GPS as an intermediate step.
pub fn beidou_to_utc<P: LeapSecondsProvider>(
    bdt: Time<Beidou>,
    ls: &P,
) -> Result<Time<Utc>, GnssTimeError> {
    let gps = bdt.try_convert::<Gps>()?;

    gps_to_utc(gps, ls)
}

/// BeiDou -> GLONASS via UTC (requires leap-second context).
pub fn beidou_to_glonass<P: LeapSecondsProvider>(
    bdt: Time<Beidou>,
    ls: &P,
) -> Result<Time<Glonass>, GnssTimeError> {
    let utc = beidou_to_utc(bdt, ls)?;

    utc_to_glonass(utc)
}

////////////////////////////////////////////////////////////////////////////////
// UTC -> GLONASS, GPS, Galielo, BeiDou
////////////////////////////////////////////////////////////////////////////////

/// Converts UTC -> GLONASS (without leap-second context).
///
/// # Errors
///
/// [`GnssTimeError::Overflow`] — if UTC is earlier than the GLONASS epoch
/// (1996-01-01 UTC(SU)).
pub fn utc_to_glonass(utc: Time<Utc>) -> Result<Time<Glonass>, GnssTimeError> {
    let glo_ns = (utc.as_nanos() as i128) - (GLONASS_FROM_UTC_EPOCH_NS as i128);

    if glo_ns < 0 || glo_ns > u64::MAX as i128 {
        return Err(GnssTimeError::Overflow);
    }

    Ok(Time::<Glonass>::from_nanos(glo_ns as u64))
}

/// Converts UTC -> GPS.
///
/// Requires an explicit [`LeapSecondsProvider`] context.
///
/// # Accuracy at leap-second insertion
///
/// During the 1-second leap-second insertion window, the result may be off by
/// 1 second. For all other instants, the result is exact.
///
/// # Errors
///
/// [`GnssTimeError::Overflow`] — the result does not fit into `u64`.
pub fn utc_to_gps<P: LeapSecondsProvider>(
    utc: Time<Utc>,
    ls: &P,
) -> Result<Time<Gps>, GnssTimeError> {
    // Two-pass computation for correct leap-second boundary handling.
    //
    // Pass 1: approximate TAI assuming GPS-UTC = 0.
    // This underestimates TAI by at most (current GPS-UTC) seconds
    // near boundary conditions.
    let approx_tai_ns =
        (utc.as_nanos() as i128) - (UTC_TO_GPS_EPOCH_NS as i128) + 19_000_000_000_i128;

    let tai1 = if approx_tai_ns >= 0 && approx_tai_ns <= u64::MAX as i128 {
        Time::<Tai>::from_nanos(approx_tai_ns as u64)
    } else {
        Time::<Tai>::EPOCH
    };

    let n1 = ls.tai_minus_utc_at(tai1);

    // Pass 2: refinement using n1, resolving boundary ambiguity.
    let refined_tai_ns = (utc.as_nanos() as i128) - (UTC_TO_GPS_EPOCH_NS as i128)
        + (n1 as i128 * 1_000_000_000_i128);

    let tai2 = if refined_tai_ns >= 0 && refined_tai_ns <= u64::MAX as i128 {
        Time::<Tai>::from_nanos(refined_tai_ns as u64)
    } else {
        tai1
    };

    let n = ls.tai_minus_utc_at(tai2);

    let gps_ns = (utc.as_nanos() as i128) + ((n - 19) as i128 * 1_000_000_000_i128)
        - (UTC_TO_GPS_EPOCH_NS as i128);
    if gps_ns < 0 || gps_ns > u64::MAX as i128 {
        return Err(GnssTimeError::Overflow);
    }

    Ok(Time::<Gps>::from_nanos(gps_ns as u64))
}

/// Converts UTC -> Galileo (requires leap-second context).
pub fn utc_to_galileo<P: LeapSecondsProvider>(
    utc: Time<Utc>,
    ls: &P,
) -> Result<Time<Galileo>, GnssTimeError> {
    let gps = utc_to_gps(utc, ls)?;

    gps.try_convert::<Galileo>()
}

/// Converts UTC -> BeiDou (requires leap-second context).
pub fn utc_to_beidou<P: LeapSecondsProvider>(
    utc: Time<Utc>,
    ls: &P,
) -> Result<Time<Beidou>, GnssTimeError> {
    let gps = utc_to_gps(utc, ls)?;

    gps.try_convert::<Beidou>()
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use std::string::ToString;

    use super::*;
    use crate::scale::Gps;

    #[test]
    fn test_utc_to_gps_epoch_offset_is_252892800_seconds() {
        assert_eq!(UTC_TO_GPS_EPOCH_NS / 1_000_000_000, 252_892_800);
    }

    #[test]
    fn test_utc_to_gps_epoch_offset_is_2927_days() {
        assert_eq!(UTC_TO_GPS_EPOCH_NS / 1_000_000_000 / 86_400, 2927);
    }

    #[test]
    fn test_glonass_epoch_offset_from_utc_epoch_is_correct() {
        // 757_371_600 s = 8766 days * 86400 - 3h
        // = (days from 1972-01-01 to 1996-01-01) * 86400 - 10800
        assert_eq!(GLONASS_FROM_UTC_EPOCH_NS / 1_000_000_000, 757_371_600);
    }

    #[test]
    fn test_builtin_table_is_sorted() {
        let entries = LeapSeconds::builtin().entries();

        for w in entries.windows(2) {
            assert!(
                w[0].tai_nanos < w[1].tai_nanos,
                "table not sorted at {:?}",
                w
            );
        }
    }

    #[test]
    fn test_builtin_table_starts_with_tai_minus_utc_19() {
        assert_eq!(BUILTIN_TABLE[0].tai_minus_utc, 19);
    }

    #[test]
    fn test_builtin_table_ends_with_tai_minus_utc_37() {
        let last = *BUILTIN_TABLE.last().unwrap();

        assert_eq!(last.tai_minus_utc, 37);
    }

    #[test]
    fn test_builtin_table_has_monotone_increasing_tai_minus_utc() {
        let entries = LeapSeconds::builtin().entries();

        for w in entries.windows(2) {
            assert!(
                w[1].tai_minus_utc == w[0].tai_minus_utc + 1,
                "expected each entry to increment by 1"
            );
        }
    }

    #[test]
    fn test_lookup_at_tai_zero_returns_19() {
        let ls = LeapSeconds::builtin();

        assert_eq!(ls.tai_minus_utc_at(Time::<Tai>::EPOCH), 19);
    }

    #[test]
    fn test_lookup_at_max_tai_returns_last_value() {
        let ls = LeapSeconds::builtin();

        assert_eq!(ls.tai_minus_utc_at(Time::<Tai>::MAX), 37);
    }

    #[test]
    fn test_lookup_at_exact_2017_threshold_returns_37() {
        let ls = LeapSeconds::builtin();
        // Threshold TAI value for 2017-01-01 = 1_167_264_037_000_000_000
        let tai = Time::<Tai>::from_nanos(1_167_264_037_000_000_000);

        assert_eq!(ls.tai_minus_utc_at(tai), 37);
    }

    #[test]
    fn test_lookup_one_ns_before_2017_threshold_returns_36() {
        let ls = LeapSeconds::builtin();
        let tai = Time::<Tai>::from_nanos(1_167_264_037_000_000_000 - 1);

        assert_eq!(ls.tai_minus_utc_at(tai), 36);
    }

    #[test]
    fn test_lookup_at_1999_threshold_returns_32() {
        let ls = LeapSeconds::builtin();
        // Threshold TAI value for 1999-01-01 = 599_184_032_000_000_000
        let tai = Time::<Tai>::from_nanos(599_184_032_000_000_000);

        assert_eq!(ls.tai_minus_utc_at(tai), 32);
    }

    #[test]
    fn test_lookup_one_ns_before_1999_threshold_returns_31() {
        let ls = LeapSeconds::builtin();
        let tai = Time::<Tai>::from_nanos(599_184_032_000_000_000 - 1);

        assert_eq!(ls.tai_minus_utc_at(tai), 31);
    }

    #[test]
    fn test_gps_utc_gps_roundtrip_at_gps_epoch() {
        let ls = LeapSeconds::builtin();
        let gps = Time::<Gps>::EPOCH;
        let utc = gps_to_utc(gps, &ls).unwrap();
        let back = utc_to_gps(utc, &ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_gps_utc_gps_roundtrip_at_2020() {
        let ls = LeapSeconds::builtin();
        // GPS 2020-01-01 ≈ week 2086
        let gps = Time::<Gps>::from_week_tow(2086, 0.0).unwrap();
        let utc = gps_to_utc(gps, &ls).unwrap();
        let back = utc_to_gps(utc, &ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_gps_epoch_utc_is_correct_offset_from_utc_epoch() {
        let ls = LeapSeconds::builtin();
        // At GPS epoch (1980-01-06) TAI-UTC = 19, GPS-UTC = 0
        // UTC nanos = GPS nanos + UTC_TO_GPS_EPOCH_NS = 0 +
        // 252_892_800_000_000_000
        let utc = gps_to_utc(Time::<Gps>::EPOCH, &ls).unwrap();

        assert_eq!(utc.as_nanos(), 252_892_800_000_000_000);
    }

    // Checking GPS-UTC = 18 at 2017-01-01 00:00:00 UTC.
    //
    // GPS at 2017-01-01 (unix=1483228800):
    //   GPS_s = (1483228800 - 315964800) + (37-19) = 1167264000 + 18 = 1167264018
    // UTC nanos from UTC_epoch = 16437 days * 86400 * 1e9 =
    // 1_420_156_800_000_000_000
    #[test]
    fn test_gps_minus_utc_is_18s_at_2017_01_01() {
        let ls = LeapSeconds::builtin();
        // GPS seconds for 2017-01-01 00:00:00 UTC
        // = (unix - GPS_EPOCH_UNIX) + (TAI-UTC - 19) = (1483228800 - 315964800) + 18
        let gps_s: u64 = 1_167_264_000 + 18;
        let gps = Time::<Gps>::from_seconds(gps_s);
        let utc = gps_to_utc(gps, &ls).unwrap();

        // UTC nanos for 2017-01-01 = 16437 days * 86400 * 1e9
        let expected_utc_ns: u64 = 16_437 * 86_400 * 1_000_000_000;

        assert_eq!(utc.as_nanos(), expected_utc_ns);
    }

    // Check GPS-UTC = 13 on 1999-01-01 00:00:00 UTC.
    #[test]
    fn test_gps_minus_utc_is_13s_at_1999_01_01() {
        let ls = LeapSeconds::builtin();
        // GPS_s = (915148800 - 315964800) + (32 - 19) = 599184000 + 13 = 599184013
        let gps = Time::<Gps>::from_seconds(599_184_013);
        let utc = gps_to_utc(gps, &ls).unwrap();

        // UTC from UTC epoch to 1999-01-01:
        // days_from_unix(1999-01-01) - days_from_unix(1972-01-01)
        // = 10592 - 730 = 9862 days (verified below)
        // UTC_s = 9862 * 86400 = 851_948_800
        let expected_utc_s: u64 = 9_862 * 86_400;

        assert_eq!(utc.as_seconds(), expected_utc_s);
    }

    // 1998-12-31 → 1999-01-01: TAI-UTC changes 31 → 32, GPS-UTC 12 → 13.
    //
    // GPS jumps from ...011 to ...013 (there is no ...012 in real UTC time).
    #[test]
    fn test_leap_second_transition_1999_gps_jumps_by_2s() {
        let ls = LeapSeconds::builtin();

        // 1 second before transition: 1998-12-31 23:59:59 UTC
        // unix = 915148799, TAI-UTC = 31 (old value)
        // GPS_s = (915148799 - 315964800) + 12 = 599183999 + 12 = 599184011
        let gps_before = Time::<Gps>::from_seconds(599_184_011);

        // Immediately after: 1999-01-01 00:00:00 UTC
        // unix = 915148800, TAI-UTC = 32 (new value)
        // GPS_s = (915148800 - 315964800) + 13 = 599184000 + 13 = 599184013
        let gps_after = Time::<Gps>::from_seconds(599_184_013);

        // Both should convert correctly
        let utc_before = gps_to_utc(gps_before, &ls).unwrap();
        let utc_after = gps_to_utc(gps_after, &ls).unwrap();

        // UTC-after - UTC-before = 1 second (leap second insertion adjusts the scale)
        let diff = (utc_after - utc_before).as_seconds();

        assert_eq!(diff, 1, "GPS jumped 2s but UTC advanced 1s (leap second)");
    }

    // 2016-12-31 → 2017-01-01: TAI-UTC 36 → 37, GPS-UTC 17 → 18.
    #[test]
    fn test_leap_second_transition_2017_gps_jumps_by_2s() {
        let ls = LeapSeconds::builtin();
        // 1 second before: unix = 1483228799,
        // GPS_s = (1483228799 - 315964800) + 17
        let gps_before = Time::<Gps>::from_seconds(1_167_263_999 + 17);
        // Immediately after: unix = 1483228800,
        // GPS_s = (1483228800 - 315964800) + 18
        let gps_after = Time::<Gps>::from_seconds(1_167_264_000 + 18);
        let utc_before = gps_to_utc(gps_before, &ls).unwrap();
        let utc_after = gps_to_utc(gps_after, &ls).unwrap();
        let diff = (utc_after - utc_before).as_seconds();

        assert_eq!(diff, 1, "GPS jumped 2s but UTC advanced 1s");
    }

    #[test]
    fn test_glonass_epoch_to_utc_gives_correct_nanos() {
        // GLONASS epoch = 1996-01-01 00:00:00 UTC(SU)
        // which corresponds to 1995-12-31 21:00:00 UTC
        //
        // UTC offset from UTC epoch:
        // (days to 1995-12-31) * 86400 + 21h * 3600 = ...
        // Verified via GLONASS_FROM_UTC_EPOCH_NS constant
        let utc = glonass_to_utc(Time::<Glonass>::EPOCH).unwrap();

        assert_eq!(utc.as_nanos(), GLONASS_FROM_UTC_EPOCH_NS as u64);
    }

    #[test]
    fn test_utc_to_glonass_epoch_gives_zero() {
        let utc = Time::<Utc>::from_nanos(GLONASS_FROM_UTC_EPOCH_NS as u64);
        let glo = utc_to_glonass(utc).unwrap();

        assert_eq!(glo, Time::<Glonass>::EPOCH);
    }

    #[test]
    fn test_glonass_utc_glonass_roundtrip() {
        let glo = Time::<Glonass>::from_day_tod(10_000, 43_200.0).unwrap();
        let utc = glonass_to_utc(glo).unwrap();
        let back = utc_to_glonass(utc).unwrap();

        assert_eq!(glo, back);
    }

    #[test]
    fn test_utc_before_glonass_epoch_returns_error() {
        // UTC epoch (1972-01-01) is earlier than GLONASS epoch (1996),
        // so conversion results in underflow/overflow
        let utc = Time::<Utc>::EPOCH;

        assert!(matches!(utc_to_glonass(utc), Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_glonass_offset_is_exactly_3_hours_less_than_day_boundary() {
        // Offset = 8766 days * 86400 - 3*3600 (exactly 3 hours before midnight
        // 1996-01-01 UTC)
        let three_hours_ns: i64 = 3 * 3_600 * 1_000_000_000;
        let days_ns: i64 = 8766 * 86_400 * 1_000_000_000;

        assert_eq!(GLONASS_FROM_UTC_EPOCH_NS, days_ns - three_hours_ns);
    }

    #[test]
    fn test_gps_to_glonass_to_gps_roundtrip() {
        let ls = LeapSeconds::builtin();
        // GPS time in 2020 (after the last leap second in 2017)
        let gps = Time::<Gps>::from_week_tow(2100, 86400.0).unwrap();
        let glo = gps_to_glonass(gps, &ls).unwrap();
        let back = glonass_to_gps(glo, &ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_custom_provider_works() {
        struct Always37;

        impl LeapSecondsProvider for Always37 {
            fn tai_minus_utc_at(
                &self,
                _: Time<Tai>,
            ) -> i32 {
                37
            }
        }

        let gps = Time::<Gps>::from_seconds(1_000_000_000);
        let utc = gps_to_utc(gps, &Always37).unwrap();
        let back = utc_to_gps(utc, &Always37).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_empty_table_returns_fallback_19() {
        static EMPTY: [LeapEntry; 0] = [];

        let ls = LeapSeconds::from_table(&EMPTY);

        assert_eq!(
            ls.tai_minus_utc_at(Time::<Tai>::from_seconds(1_000_000)),
            19
        );
    }
}
