//! # Leap seconds — conversion context
//!
//! ## Why this is an explicit parameter, not global state
//!
//! ```text
//! // Hidden state — bad
//! let utc = gps.to_utc(); // where do the leap seconds come from?
//!
//! // Explicit context — good
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

/// Maximum number of entries in a [`RuntimeLeapSeconds`] buffer.
///
/// 64 entries is far beyond any plausible number of leap seconds in the
/// foreseeable future (current count from 1972: 27 events).
pub const RUNTIME_CAPACITY: usize = 64;

static BUILTIN_LEAP_SECONDS: LeapSeconds = LeapSeconds {
    entries: &BUILTIN_TABLE,
};

/// Nanoseconds from the UTC epoch (1972-01-01) to the GLONASS epoch
/// (1995-12-31 21:00:00 UTC).
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

/// Error returned by [`RuntimeLeapSeconds::try_extend`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use = "handle the extension error; ignoring it means the table was not updated"]
#[non_exhaustive]
pub enum LeapExtendError {
    /// The new entry's `tai_nanos` is not strictly greater than the last
    /// existing entry — the table would become unsorted.
    NotStrictlyAscending,

    /// The new entry's `tai_minus_utc` is not exactly one more than the last
    /// existing entry — every leap second must increment the counter by 1.
    NonUnitIncrement,

    /// The runtime buffer is full; no more entries can be appended.
    BufferFull,
}

/// One leap-second table entry.
///
/// Starting from `tai_minus_utc` (internal TAI nanoseconds), `TAI - UTC =
/// tai_minus_utc` seconds.
///
/// Strict contract: the table must be sorted by `tai_nanos` in ascending order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
/// use gnss_time::{gps_to_utc, DurationParts, Gps, LeapSeconds, LeapSecondsProvider, Time};
///
/// // Built-in table (up to 2017)
/// let ls = LeapSeconds::builtin();
///
/// let gps = Time::<Gps>::from_week_tow(
///     1981,
///     DurationParts {
///         seconds: 0,
///         nanos: 0,
///     },
/// )
/// .unwrap();
/// let utc = gps_to_utc(gps, &ls).unwrap();
/// // GPS leads UTC by 18 seconds in this period
/// ```
pub struct LeapSeconds {
    entries: &'static [LeapEntry], // (Unix seconds, TAI-UTC)
}

/// A heap-free, fixed-capacity leap-second table for embedded / receiver use.
///
/// Suitable for GNSS receivers that receive the current leap-second count from
/// the GPS navigation message and need an up-to-date table without any heap
/// allocation.
///
/// Start with [`from_builtin`](Self::from_builtin) to pre-populate the
/// compile-time snapshot, then call [`try_extend`](Self::try_extend) whenever
/// the receiver almanac reports a new event.
///
/// # Capacity
///
/// Holds up to [`RUNTIME_CAPACITY`] (64) entries.
///
/// # Example
///
/// ```rust
/// use gnss_time::{LeapEntry, LeapSecondsProvider, RuntimeLeapSeconds, Tai, Time};
///
/// let mut rt = RuntimeLeapSeconds::from_builtin();
///
/// // Hypothetical future event (illustrative only).
/// // rt.try_extend(LeapEntry::new(9_999_999_999_000_000_000, 38)).unwrap();
///
/// assert_eq!(rt.current_tai_minus_utc(), 37);
/// ```
#[derive(Debug)]
pub struct RuntimeLeapSeconds {
    buf: [LeapEntry; RUNTIME_CAPACITY],
    len: usize,
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
    #[must_use]
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
    /// Covers all 19 entries in the GPS era (1980-01-06 … 2017-01-01).
    ///
    /// **Last verified:** IERS Bulletin C 70 (December 2024) — no new leap
    /// seconds scheduled through June 2025. Status as of May 2026: TAI−UTC =
    /// 37, unchanged.
    ///
    /// Source: [IERS Bulletin C](https://www.iers.org/IERS/EN/Publications/Bulletins/bulletins.html)
    #[inline]
    #[must_use]
    pub fn builtin() -> &'static LeapSeconds {
        &BUILTIN_LEAP_SECONDS
    }

    /// Creates a table from a custom static slice.
    ///
    /// This is an alias for [`from_table`](Self::from_table), provided for API
    /// symmetry with [`RuntimeLeapSeconds::from_slice`].
    ///
    /// # Requirements
    ///
    /// `entries` must be sorted by `tai_nanos` in strictly ascending order and
    /// each consecutive `tai_minus_utc` must increment by exactly 1.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::{LeapEntry, LeapSeconds};
    ///
    /// static MY_TABLE: [LeapEntry; 1] = [LeapEntry::new(0, 37)];
    /// let ls = LeapSeconds::from_slice(&MY_TABLE);
    ///
    /// assert_eq!(ls.len(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_slice(entries: &'static [LeapEntry]) -> Self {
        Self { entries }
    }

    /// Creates a table from a custom static slice (canonical name).
    ///
    /// # Requirements
    ///
    /// `entries` must be sorted by `tai_nanos` in ascending order.
    #[inline]
    #[must_use]
    pub const fn from_table(entries: &'static [LeapEntry]) -> Self {
        Self { entries }
    }

    /// Returns the number of entries in the table.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the table is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns all table entries (for inspection / serialization).
    #[inline]
    #[must_use]
    pub fn entries(&self) -> &[LeapEntry] {
        self.entries
    }

    /// Returns the TAI timestamp of the most recent leap-second event.
    ///
    /// Returns `None` when the table contains only the base entry (threshold
    /// = 0) or is empty — in those cases there is no recorded event timestamp.
    ///
    /// Useful for diagnostics: compare against the current time to detect
    /// whether the table may be stale.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::LeapSeconds;
    ///
    /// let ls = LeapSeconds::builtin();
    /// let last = ls.last_update().expect("builtin table is non-empty");
    ///
    /// // 2017-01-01 TAI threshold
    /// assert_eq!(last.as_nanos(), 1_167_264_037_000_000_000);
    /// ```
    #[inline]
    #[must_use]
    pub const fn last_update(&self) -> Option<Time<Tai>> {
        if self.entries.len() <= 1 {
            return None;
        }

        let last = &self.entries[self.entries.len() - 1];

        Some(Time::<Tai>::from_nanos(last.tai_nanos))
    }

    /// Returns the current TAI − UTC value (the `tai_minus_utc` of the last
    /// entry), or 19 for an empty table.
    ///
    /// Equivalent to `tai_minus_utc_at(Time::<Tai>::MAX)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::LeapSeconds;
    ///
    /// assert_eq!(LeapSeconds::builtin().current_tai_minus_utc(), 37);
    /// ```
    #[inline]
    #[must_use]
    pub const fn current_tai_minus_utc(&self) -> i32 {
        if self.entries.is_empty() {
            return 19;
        }

        self.entries[self.entries.len() - 1].tai_minus_utc
    }
}

impl RuntimeLeapSeconds {
    /// Creates an empty runtime table.
    ///
    /// Call [`try_extend`](Self::try_extend) or use
    /// [`from_builtin`](Self::from_builtin) before performing conversions.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            buf: [LeapEntry::new(0, 0); RUNTIME_CAPACITY],
            len: 0,
        }
    }

    /// Creates a runtime table pre-populated from built-in static table.
    ///
    /// This is the recommended starting point for receivers: begin with the
    /// compile-time snapshot and extend when the almanac reports new data.
    ///
    /// # Panics
    ///
    /// Panics if `BUILTIN_YABLE.len() > RUNTIME_CAPACITY` (cannot happen with
    /// current constants, but asserted for correctness).
    #[must_use]
    pub fn from_builtin() -> Self {
        assert!(
            BUILTIN_TABLE.len() <= RUNTIME_CAPACITY,
            "BUILTIN_TABLE exceeds RUNTIME_CAPACITY"
        );

        let mut rt = Self::new();

        for &entry in BUILTIN_TABLE.iter() {
            rt.buf[rt.len] = entry;
            rt.len += 1;
        }

        rt
    }

    /// Creates a runtime table from a slice of entries.
    ///
    /// Mirrors [`LeapSeconds::from_slice`] for contexts where a mutable /
    /// extendable table is needed.
    ///
    /// # Errors
    ///
    /// Returns [`LeapExtendError::BufferFull`] if `entries.len() >
    /// RUNTIME_CAPACITY`.
    pub fn from_slice(entries: &[LeapEntry]) -> Result<Self, LeapExtendError> {
        if entries.len() > RUNTIME_CAPACITY {
            return Err(LeapExtendError::BufferFull);
        }

        let mut rt = Self::new();

        for &entry in entries {
            rt.buf[rt.len] = entry;
            rt.len += 1;
        }

        Ok(rt)
    }

    /// Appends a new leap-second event to the runtime table.
    ///
    /// Internally, the table is treated as a strictly ordered sequence of
    /// leap-second transitions. Each new entry must extend the sequence
    /// without breaking its monotonic structure.
    ///
    /// # Validation
    ///
    /// The new entry must satisfy:
    /// - `entry.tai_nanos > last().tai_nanos` — strictly ascending order
    /// - `entry.tai_minus_utc == last().tai_minus_utc + 1` — unit increment
    ///
    /// # Errors
    ///
    /// - [`LeapExtendError::NotStrictlyAscending`] — threshold not increasing
    /// - [`LeapExtendError::NonUnitIncrement`] — value does not increment by 1
    /// - [`LeapExtendError::BufferFull`] — capacity exhausted
    ///
    /// # Notes
    ///
    /// This method does not attempt to validate whether the provided entry
    /// corresponds to a *real* leap second published by official sources.
    /// It only enforces internal consistency of the sequence.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::{LeapEntry, RuntimeLeapSeconds};
    ///
    /// let mut rt = RuntimeLeapSeconds::from_builtin();
    ///
    /// // Hypothetical future leap second (not a real event).
    /// rt.try_extend(LeapEntry::new(9_999_999_999_000_000_000, 38))
    ///     .unwrap();
    ///
    /// assert_eq!(rt.current_tai_minus_utc(), 38);
    /// assert_eq!(rt.len(), 20);
    /// ```
    pub fn try_extend(
        &mut self,
        entry: LeapEntry,
    ) -> Result<(), LeapExtendError> {
        // Prevent writing past the fixed buffer.
        // This keeps the structure allocation-free and predictable.
        if self.len >= RUNTIME_CAPACITY {
            return Err(LeapExtendError::BufferFull);
        }

        // If there is at least one entry, validate against the last one.
        if self.len > 0 {
            let last = &self.buf[self.len - 1];

            // Enforce strict monotonicity in time.
            // Equal or smaller timestamps would break ordering assumptions.
            if entry.tai_nanos <= last.tai_nanos {
                return Err(LeapExtendError::NotStrictlyAscending);
            }

            // Enforce +1 step in TAI−UTC offset.
            // Anything else would violate leap second semantics.
            if entry.tai_minus_utc != last.tai_minus_utc + 1 {
                return Err(LeapExtendError::NonUnitIncrement);
            }
        }

        self.buf[self.len] = entry;
        self.len += 1;

        Ok(())
    }

    /// Returns the number of entries currently in the table.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the table has no entries.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns all live entries as a slice.
    #[inline]
    #[must_use]
    pub fn entries(&self) -> &[LeapEntry] {
        &self.buf[..self.len]
    }

    /// Returns the TAI timestamp of the most recent event, or `None` for a
    /// single-entry or empty table.
    #[inline]
    #[must_use]
    pub const fn last_update(&self) -> Option<Time<Tai>> {
        if self.len <= 1 {
            return None;
        }

        Some(Time::<Tai>::from_nanos(self.buf[self.len - 1].tai_nanos))
    }

    /// Returns the current TAI - UTC value (last entry), or 19 for an empty
    /// table.
    #[inline]
    #[must_use]
    pub const fn current_tai_minus_utc(&self) -> i32 {
        if self.len == 0 {
            return 19;
        }

        self.buf[self.len - 1].tai_minus_utc
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

impl LeapSecondsProvider for RuntimeLeapSeconds {
    fn tai_minus_utc_at(
        &self,
        tai: Time<Tai>,
    ) -> i32 {
        let entries = self.entries();
        let nanos = tai.as_nanos();

        if entries.is_empty() {
            return 19;
        }

        match entries.binary_search_by_key(&nanos, |e| e.tai_nanos) {
            Ok(i) => entries[i].tai_minus_utc,
            Err(0) => entries[0].tai_minus_utc,
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

impl core::fmt::Display for LeapExtendError {
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        match self {
            LeapExtendError::NotStrictlyAscending => {
                f.write_str("new entry tai_nanos is not strictly greater than the last entry")
            }
            LeapExtendError::NonUnitIncrement => {
                f.write_str("new entry tai_minus_utc be exactly one more tham the last entry")
            }
            LeapExtendError::BufferFull => {
                f.write_str("runtime leap-second buffer is full; cannot add more entries")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LeapExtendError {}

impl Default for RuntimeLeapSeconds {
    fn default() -> Self {
        Self::new()
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use std::string::ToString;

    use super::*;
    use crate::{scale::Gps, DurationParts};

    #[test]
    fn test_utc_to_gps_epoch_offset_is_252892800_seconds() {
        assert_eq!(UTC_TO_GPS_EPOCH_NS / 1_000_000_000, 252_892_800);
    }

    #[test]
    fn test_glonass_epoch_offset_is_757371600_seconds() {
        assert_eq!(GLONASS_FROM_UTC_EPOCH_NS / 1_000_000_000, 757_371_600);
    }

    #[test]
    fn test_builtin_table_length() {
        assert_eq!(LeapSeconds::builtin().len(), 19);
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
        assert_eq!(LeapSeconds::builtin().entries()[0].tai_minus_utc, 19);
    }

    #[test]
    fn test_builtin_table_ends_with_tai_minus_utc_37() {
        let last = *LeapSeconds::builtin().entries().last().unwrap();
        assert_eq!(last.tai_minus_utc, 37);
    }

    #[test]
    fn test_builtin_table_has_monotone_increasing_tai_minus_utc() {
        let entries = LeapSeconds::builtin().entries();

        for w in entries.windows(2) {
            assert_eq!(
                w[1].tai_minus_utc,
                w[0].tai_minus_utc + 1,
                "expected each entry to increment by 1"
            );
        }
    }

    // Cross-reference against raw IERS Bulletin C data.
    //
    // Each TAI threshold is independently recomputed from the Unix event
    // timestamp using the canonical formula and compared to the compiled
    // table.
    #[test]
    fn test_builtin_table_matches_iers_bulletin_c() {
        const GPS_EPOCH_UNIX: u64 = 315_964_800;

        // (unix_event_timestamp, expected_tai_minus_utc)
        let iers_events: &[(u64, i32)] = &[
            (362_793_600, 20),   // 1981-07-01
            (394_329_600, 21),   // 1982-07-01
            (425_865_600, 22),   // 1983-07-01
            (489_024_000, 23),   // 1985-07-01
            (567_993_600, 24),   // 1988-01-01
            (631_152_000, 25),   // 1990-01-01
            (662_688_000, 26),   // 1991-01-01
            (709_948_800, 27),   // 1992-07-01
            (741_484_800, 28),   // 1993-07-01
            (773_020_800, 29),   // 1994-07-01
            (820_454_400, 30),   // 1996-01-01
            (867_715_200, 31),   // 1997-07-01
            (915_148_800, 32),   // 1999-01-01
            (1_136_073_600, 33), // 2006-01-01
            (1_230_768_000, 34), // 2009-01-01
            (1_341_100_800, 35), // 2012-07-01
            (1_435_708_800, 36), // 2015-07-01
            (1_483_228_800, 37), // 2017-01-01
        ];

        let entries = LeapSeconds::builtin().entries();

        // Entry 0 is the base value at GPS epoch.
        assert_eq!(entries[0].tai_nanos, 0);
        assert_eq!(entries[0].tai_minus_utc, 19);

        // Entries 1..18 must match the IERS events exactly.
        for (idx, &(unix, expected_n)) in iers_events.iter().enumerate() {
            let gps_s = unix - GPS_EPOCH_UNIX;
            let expected_threshold = (gps_s + expected_n as u64) * 1_000_000_000;
            let entry = &entries[idx + 1];

            assert_eq!(
                entry.tai_nanos,
                expected_threshold,
                "threshold mismatch at IERS event {} (unix={})",
                idx + 1,
                unix
            );
            assert_eq!(
                entry.tai_minus_utc,
                expected_n,
                "tai_minus_utc mismatch at IERS event {} (unix={})",
                idx + 1,
                unix
            );
        }
    }

    #[test]
    fn test_last_update_builtin_is_2017_threshold() {
        let last = LeapSeconds::builtin()
            .last_update()
            .expect("builtin must have last_update");

        assert_eq!(last.as_nanos(), 1_167_264_037_000_000_000);
    }

    #[test]
    fn test_last_update_single_entry_is_none() {
        static SINGLE: [LeapEntry; 1] = [LeapEntry::new(0, 37)];
        let ls = LeapSeconds::from_slice(&SINGLE);

        assert!(ls.last_update().is_none());
    }

    #[test]
    fn test_last_update_empty_is_none() {
        static EMPTY: [LeapEntry; 0] = [];
        let ls = LeapSeconds::from_slice(&EMPTY);

        assert!(ls.last_update().is_none());
    }

    #[test]
    fn test_current_tai_minus_utc_builtin_is_37() {
        assert_eq!(LeapSeconds::builtin().current_tai_minus_utc(), 37);
    }

    #[test]
    fn test_current_tai_minus_utc_empty_is_fallback_19() {
        static EMPTY: [LeapEntry; 0] = [];
        let ls = LeapSeconds::from_slice(&EMPTY);

        assert_eq!(ls.current_tai_minus_utc(), 19);
    }

    #[test]
    fn test_from_slice_and_from_table_are_equivalent() {
        static TABLE: [LeapEntry; 2] = [LeapEntry::new(0, 19), LeapEntry::new(1_000_000, 20)];

        let ls_slice = LeapSeconds::from_slice(&TABLE);
        let ls_table = LeapSeconds::from_table(&TABLE);

        assert_eq!(ls_slice.len(), ls_table.len());
        assert_eq!(
            ls_slice.entries()[0].tai_nanos,
            ls_table.entries()[0].tai_nanos
        );
    }

    #[test]
    fn test_lookup_at_tai_zero_returns_19() {
        let ls = LeapSeconds::builtin();
        assert_eq!(ls.tai_minus_utc_at(Time::<Tai>::EPOCH), 19);
    }

    #[test]
    fn test_lookup_at_max_tai_returns_37() {
        let ls = LeapSeconds::builtin();
        assert_eq!(ls.tai_minus_utc_at(Time::<Tai>::MAX), 37);
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
        let gps = Time::<Gps>::from_week_tow(
            2086,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap();
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
        let glo = Time::<Glonass>::from_day_tod(
            10_000,
            DurationParts {
                seconds: 43_200,
                nanos: 0,
            },
        )
        .unwrap();
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
        let gps = Time::<Gps>::from_week_tow(
            2100,
            DurationParts {
                seconds: 86400,
                nanos: 0,
            },
        )
        .unwrap();
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

    #[test]
    fn test_runtime_from_builtin_has_19_entries() {
        assert_eq!(RuntimeLeapSeconds::from_builtin().len(), 19);
    }

    #[test]
    fn test_runtime_from_builtin_current_is_37() {
        assert_eq!(
            RuntimeLeapSeconds::from_builtin().current_tai_minus_utc(),
            37
        );
    }

    #[test]
    fn test_runtime_try_extend_valid() {
        let mut rt = RuntimeLeapSeconds::from_builtin();
        rt.try_extend(LeapEntry::new(9_999_999_999_000_000_000, 38))
            .unwrap();

        assert_eq!(rt.len(), 20);
        assert_eq!(rt.current_tai_minus_utc(), 38);
    }

    #[test]
    fn test_runtime_try_extend_last_update_updated() {
        let mut rt = RuntimeLeapSeconds::from_builtin();
        rt.try_extend(LeapEntry::new(9_999_999_999_000_000_000, 38))
            .unwrap();

        let last = rt.last_update().unwrap();
        assert_eq!(last.as_nanos(), 9_999_999_999_000_000_000);
    }

    #[test]
    fn test_runtime_try_extend_not_ascending_error() {
        let mut rt = RuntimeLeapSeconds::from_builtin();
        // Same threshold as last builtin entry — not strictly ascending.
        let err = rt
            .try_extend(LeapEntry::new(1_167_264_037_000_000_000, 38))
            .unwrap_err();

        assert_eq!(err, LeapExtendError::NotStrictlyAscending);
    }

    #[test]
    fn test_runtime_try_extend_non_unit_increment_error() {
        let mut rt = RuntimeLeapSeconds::from_builtin();
        // Skips to 39 instead of 38.
        let err = rt
            .try_extend(LeapEntry::new(9_999_999_999_000_000_000, 39))
            .unwrap_err();

        assert_eq!(err, LeapExtendError::NonUnitIncrement);
    }

    #[test]
    fn test_runtime_from_slice_too_large_returns_buffer_full() {
        let big: std::vec::Vec<LeapEntry> = (0..RUNTIME_CAPACITY + 1)
            .map(|i| LeapEntry::new(i as u64 * 1_000_000_000, 19 + i as i32))
            .collect();
        let err = RuntimeLeapSeconds::from_slice(&big).unwrap_err();

        assert_eq!(err, LeapExtendError::BufferFull);
    }

    #[test]
    fn test_runtime_provider_matches_static_at_all_thresholds() {
        let rt = RuntimeLeapSeconds::from_builtin();
        let ls = LeapSeconds::builtin();

        let test_nanos: &[u64] = &[
            0,
            46_828_820_000_000_000,
            599_184_032_000_000_000,
            1_167_264_037_000_000_000,
            u64::MAX,
        ];

        for &nanos in test_nanos {
            let tai = Time::<Tai>::from_nanos(nanos);
            assert_eq!(
                rt.tai_minus_utc_at(tai),
                ls.tai_minus_utc_at(tai),
                "mismatch at tai_nanos={}",
                nanos
            );
        }
    }

    #[test]
    fn test_runtime_empty_last_update_is_none() {
        assert!(RuntimeLeapSeconds::new().last_update().is_none());
    }

    #[test]
    fn test_runtime_single_entry_last_update_is_none() {
        let mut rt = RuntimeLeapSeconds::new();
        rt.try_extend(LeapEntry::new(0, 19)).unwrap();
        assert!(rt.last_update().is_none());
    }

    #[test]
    fn test_gps_utc_gps_roundtrip_with_runtime_table() {
        let rt = RuntimeLeapSeconds::from_builtin();
        let gps = Time::<Gps>::from_week_tow(
            2086,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap();
        let utc = gps_to_utc(gps, &rt).unwrap();
        let back = utc_to_gps(utc, &rt).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_gps_utc_roundtrip_extended_table() {
        let mut rt = RuntimeLeapSeconds::from_builtin();
        rt.try_extend(LeapEntry::new(9_999_999_999_000_000_000, 38))
            .unwrap();

        let gps = Time::<Gps>::from_week_tow(
            2086,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap();
        let utc = gps_to_utc(gps, &rt).unwrap();
        let back = utc_to_gps(utc, &rt).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_gps_epoch_utc_is_correct() {
        let ls = LeapSeconds::builtin();
        let utc = gps_to_utc(Time::<Gps>::EPOCH, &ls).unwrap();

        assert_eq!(utc.as_nanos(), 252_892_800_000_000_000);
    }

    #[test]
    fn test_custom_provider_roundtrip() {
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
}
