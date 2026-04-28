//! # Conversion matrix: the full graph of supported transformations
//!
//! This module documents and validates the **complete matrix** of allowed
//! conversions between time scales, and also provides
//! [`crate::matrix::ConversionMatrix`],
//! a runtime type for checking scale compatibility.
//!
//! ## Offset table (sources: ICD-GLONASS, IS-GPS-200, OS-SIS-ICD Galileo, BDS-SIS-ICD)
//!
//! | From \ To  | GLONASS     | GPS        | Galileo    | BeiDou     | TAI        | UTC         |
//! |------------|-------------|------------|------------|------------|------------|-------------|
//! | **GLONASS**| —           | via UTC+LS | via UTC+LS | via UTC+LS | no (ctx)   | +757371600c |
//! | **GPS**    | via UTC+LS  | —          | identity   | −14c       | +19c       | via LS      |
//! | **Galileo**| via UTC+LS  | identity   | —          | −14c       | +19c       | via LS      |
//! | **BeiDou** | via UTC+LS  | +14c       | +14c       | —          | +33c       | via LS      |
//! | **TAI**    | no (ctx)    | −19c       | −19c       | −33c       | —          | via LS      |
//! | **UTC**    | +757371600s | via LS     | via LS     | via LS     | via LS     | —           |
//!
//! Definitions:
//! - `identity` — equal nanosecond values (GPS and Galileo share the same TAI
//!   offset of 19 s)
//! - `+N s` — fixed offset, no leap seconds involved
//! - `via UTC+LS` — requires an explicit [`LeapSecondsProvider`]
//! - `via LS` — requires an explicit [`LeapSecondsProvider`]
//! - `+757371600s` — constant epoch shift between GLONASS and UTC, no leap
//!   seconds required
//! - `no (ctx)` — impossible without leap-second context (contextual scale)
//!
//! ## Conversion categories
//!
//! ### Fixed conversions (no context)
//! Use [`IntoScale`].
//! - GLONASS <-> UTC
//! - GPS <-> TAI, GPS <-> Galileo <-> GPS <-> BeiDou
//! - Galileo <-> BeiDou, Galileo <-> TAI, BeiDou <-> TAI
//!
//! ### Contextual conversions (require `LeapSecondsProvider`)
//! - GPS <-> UTC, GPS <-> GLONASS
//! - Galileo <-> UTC, Galileo <-> GLONASS
//! - BeiDou <-> UTC, BeiDou <-> GLONASS

use crate::{
    Beidou, Glonass, GnssTimeError, Gps, IntoScale, IntoScaleWith, LeapSecondsProvider, Tai, Time,
    Utc,
};

/// GPS offset relative to TAI in nanoseconds (GPS = TAI - 19 s).
pub const TAI_OFFSET_GPS_NS: i64 = 19 * 1_000_000_000;

/// Galileo offset relative to TAI in nanoseconds (GAL = TAI - 19 s).
pub const TAI_OFFSET_GALILEO_NS: i64 = 19 * 1_000_000_000;

/// BeiDou offset relative to TAI in nanoseconds (BDT = TAI - 33 s).
pub const TAI_OFFSET_BEIDOU_NS: i64 = 33 * 1_000_000_000;

/// TAI offset relative to itself (0 nanoseconds).
pub const TAI_OFFSET_TAI_NS: i64 = 0;

/// Constant epoch shift between GLONASS and UTC in nanoseconds.
/// GLONASS epoch (1996-01-01 00:00:00 UTC(SU)) is 757_371_600 seconds ahead
/// of the UTC epoch (1972-01-01).
pub const GLONASS_UTC_EPOCH_SHIFT_NS: i64 = 757_371_600 * 1_000_000_000;

/// Conversion kind between two time scales.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionKind {
    /// Fixed offset — no context required.
    Fixed,

    /// Identity mapping (GPS <-> Galileo: same nanoseconds).
    Identity,

    /// Constant epoch shift without leap-second context (GLONASS <-> UTC).
    EpochShift,

    /// Requires [`LeapSecondsProvider`].
    Contextual,

    /// Same scale (no conversion needed).
    SameScale,
}

/// Runtime time-scale identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScaleId {
    /// GLONASS time scale.
    Glonass,

    /// GPS time scale.
    Gps,

    /// Galileo time scale.
    Galileo,

    /// BeiDou time scale.
    Beidou,

    /// International Atomic Time.
    Tai,

    /// Coordinated Universal Time.
    Utc,
}

/// Conversion matrix: documents and validates all allowed routes between the
/// supported time scales.
///
/// # Example
///
/// ```rust
/// use gnss_time::{ConversionMatrix, ScaleId};
///
/// // Check that GPS <-> Galileo is a fixed conversion
/// assert!(ScaleId::Gps.is_fixed(ScaleId::Galileo));
///
/// // Check that GPS <-> UTC requires leap seconds
/// assert!(ScaleId::Gps.needs_leap_seconds(ScaleId::Utc));
///
/// // Full 6x6 matrix
/// let matrix = ConversionMatrix::new();
///
/// assert_eq!(matrix.path_count(false), 14); // fixed paths
/// assert_eq!(matrix.path_count(true), 16); // contextual paths
/// ```
pub struct ConversionMatrix;

/// Result of the end-to-end conversion BeiDou -> GPS -> GLONASS -> UTC -> TAI.
#[derive(Debug)]
pub struct ConversionChain {
    /// GLONASS time.
    pub glonass: Time<Glonass>,

    /// GPS time.
    pub gps: Time<Gps>,

    /// UTC time.
    pub utc: Time<Utc>,

    /// TAI time.
    pub tai: Time<Tai>,
}

impl ScaleId {
    /// All supported scales.
    pub const ALL: [ScaleId; 6] = [
        ScaleId::Glonass,
        ScaleId::Gps,
        ScaleId::Galileo,
        ScaleId::Beidou,
        ScaleId::Tai,
        ScaleId::Utc,
    ];

    /// Returns the ASCII name of the scale.
    pub const fn name(self) -> &'static str {
        match self {
            ScaleId::Glonass => "GLO",
            ScaleId::Gps => "GPS",
            ScaleId::Galileo => "GAL",
            ScaleId::Beidou => "BDT",
            ScaleId::Tai => "TAI",
            ScaleId::Utc => "UTC",
        }
    }

    /// Determines the conversion kind between the current scale and a target
    /// scale.
    ///
    /// # Parameters
    /// - `target` — target time scale
    ///
    /// # Returns
    /// The conversion kind: fixed, identity, epoch shift, contextual, or same
    /// scale.
    pub const fn conversion_kind(
        self,
        target: ScaleId,
    ) -> ConversionKind {
        use ConversionKind::*;
        use ScaleId::*;
        match (self, target) {
            (a, b) if a as u8 == b as u8 => SameScale,
            // GPS <-> TAI
            (Gps, Tai) | (Tai, Gps) => Fixed,
            // GPS <-> Galileo: идентичность (одинаковый TAI-офсет)
            (Gps, Galileo) | (Galileo, Gps) => Identity,
            // GPS <-> BeiDou: фиксированное ±14 секунд
            (Gps, Beidou) | (Beidou, Gps) => Fixed,
            // Galileo <-> BeiDou: фиксированное (как и GPS <-> BeiDou)
            (Galileo, Beidou) | (Beidou, Galileo) => Fixed,
            // Galileo <-> TAI, BeiDou <-> TAI
            (Galileo, Tai) | (Tai, Galileo) => Fixed,
            (Beidou, Tai) | (Tai, Beidou) => Fixed,
            // GLONASS <-> UTC: сдвиг эпохи, без високосных секунд
            (Glonass, Utc) | (Utc, Glonass) => EpochShift,
            // Все преобразования через границу UTC требуют учёта високосных секунд
            (Gps, Utc) | (Utc, Gps) => Contextual,
            (Gps, Glonass) | (Glonass, Gps) => Contextual,
            (Galileo, Utc) | (Utc, Galileo) => Contextual,
            (Galileo, Glonass) | (Glonass, Galileo) => Contextual,
            (Beidou, Utc) | (Utc, Beidou) => Contextual,
            (Beidou, Glonass) | (Glonass, Beidou) => Contextual,
            // TAI <-> UTC и TAI <-> GLONASS: контекстуально
            (Tai, Utc) | (Utc, Tai) => Contextual,
            (Tai, Glonass) | (Glonass, Tai) => Contextual,
            // Обработка всех будущих шкал по умолчанию
            _ => Contextual,
        }
    }

    /// Returns `true` if the conversion `self -> target` does not require leap
    /// second context.
    pub const fn is_fixed(
        self,
        target: ScaleId,
    ) -> bool {
        matches!(
            self.conversion_kind(target),
            ConversionKind::Fixed | ConversionKind::Identity | ConversionKind::EpochShift
        )
    }

    /// Returns `true` if the conversion requires a [`LeapSecondsProvider`].
    pub const fn needs_leap_seconds(
        self,
        target: ScaleId,
    ) -> bool {
        matches!(self.conversion_kind(target), ConversionKind::Contextual)
    }
}

impl ConversionMatrix {
    /// Creates a new conversion matrix.
    pub fn new() -> Self {
        ConversionMatrix
    }

    /// Returns the number of paths of the requested type (fixed or contextual).
    pub fn path_count(
        &self,
        contextual: bool,
    ) -> usize {
        let mut count = 0;

        for &from in &ScaleId::ALL {
            for &to in &ScaleId::ALL {
                if from != to {
                    let kind = from.conversion_kind(to);
                    let is_ctx = matches!(kind, ConversionKind::Contextual);

                    if contextual == is_ctx {
                        count += 1;
                    }
                }
            }
        }

        count
    }

    /// Returns the conversion kind for `from -> to`.
    pub fn kind(
        &self,
        from: ScaleId,
        to: ScaleId,
    ) -> ConversionKind {
        from.conversion_kind(to)
    }
}

impl Default for ConversionMatrix {
    fn default() -> Self {
        ConversionMatrix::new()
    }
}

/// Performs the conversion GPS -> BeiDou -> GLONASS -> UTC -> TAI in one call.
pub fn beidou_via_gps_to_glonass_via_utc<P: LeapSecondsProvider>(
    bdt: Time<Beidou>,
    ls: &P,
) -> Result<ConversionChain, GnssTimeError> {
    let gps: Time<Gps> = bdt.into_scale()?;
    let glo: Time<Glonass> = gps.into_scale_with(ls)?;
    let utc: Time<Utc> = glo.into_scale()?;
    let tai: Time<Tai> = gps.into_scale()?;

    Ok(ConversionChain {
        gps,
        glonass: glo,
        utc,
        tai,
    })
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use std::vec;

    use super::*;

    #[test]
    fn test_scale_id_names_are_correct() {
        assert_eq!(ScaleId::Glonass.name(), "GLO");
        assert_eq!(ScaleId::Gps.name(), "GPS");
        assert_eq!(ScaleId::Galileo.name(), "GAL");
        assert_eq!(ScaleId::Beidou.name(), "BDT");
        assert_eq!(ScaleId::Tai.name(), "TAI");
        assert_eq!(ScaleId::Utc.name(), "UTC");
    }

    #[test]
    fn test_same_scale_is_same_scale() {
        for &s in &ScaleId::ALL {
            assert_eq!(s.conversion_kind(s), ConversionKind::SameScale);
        }
    }

    #[test]
    fn test_gps_galileo_is_identity() {
        // Error
        assert_eq!(
            ScaleId::Gps.conversion_kind(ScaleId::Galileo),
            ConversionKind::Identity
        );
        assert_eq!(
            ScaleId::Galileo.conversion_kind(ScaleId::Gps),
            ConversionKind::Identity
        );
    }

    #[test]
    fn test_gps_tai_is_fixed() {
        assert_eq!(
            ScaleId::Gps.conversion_kind(ScaleId::Tai),
            ConversionKind::Fixed
        );
        assert_eq!(
            ScaleId::Tai.conversion_kind(ScaleId::Gps),
            ConversionKind::Fixed
        );
    }

    #[test]
    fn test_gps_beidou_is_fixed() {
        assert_eq!(
            ScaleId::Gps.conversion_kind(ScaleId::Beidou),
            ConversionKind::Fixed
        );
        assert_eq!(
            ScaleId::Beidou.conversion_kind(ScaleId::Gps),
            ConversionKind::Fixed
        );
    }

    #[test]
    fn test_glonass_utc_is_epoch_shift() {
        assert_eq!(
            ScaleId::Glonass.conversion_kind(ScaleId::Utc),
            ConversionKind::EpochShift
        );
        assert_eq!(
            ScaleId::Utc.conversion_kind(ScaleId::Glonass),
            ConversionKind::EpochShift
        );
    }

    #[test]
    fn test_contextual_conversions_require_leap_seconds() {
        let contextual_pairs = [
            (ScaleId::Gps, ScaleId::Utc),
            (ScaleId::Gps, ScaleId::Glonass),
            (ScaleId::Galileo, ScaleId::Utc),
            (ScaleId::Galileo, ScaleId::Glonass),
            (ScaleId::Beidou, ScaleId::Utc),
            (ScaleId::Beidou, ScaleId::Glonass),
        ];
        for (from, to) in contextual_pairs {
            assert!(
                from.needs_leap_seconds(to),
                "{:?} → {:?} should be contextual",
                from,
                to
            );
            assert!(
                to.needs_leap_seconds(from),
                "{:?} → {:?} should be contextual",
                to,
                from
            );
        }
    }

    #[test]
    fn test_fixed_conversions_dont_need_leap_seconds() {
        let fixed_pairs = [
            (ScaleId::Gps, ScaleId::Tai),
            (ScaleId::Gps, ScaleId::Galileo),
            (ScaleId::Gps, ScaleId::Beidou),
            (ScaleId::Galileo, ScaleId::Beidou),
            (ScaleId::Glonass, ScaleId::Utc),
        ];
        for (from, to) in fixed_pairs {
            assert!(from.is_fixed(to), "{:?} → {:?} should be fixed", from, to);
            assert!(to.is_fixed(from), "{:?} → {:?} should be fixed", to, from);
        }
    }

    #[test]
    fn test_tai_offset_constants_are_correct() {
        assert_eq!(TAI_OFFSET_GPS_NS, 19_000_000_000);
        assert_eq!(TAI_OFFSET_GALILEO_NS, 19_000_000_000);
        assert_eq!(TAI_OFFSET_BEIDOU_NS, 33_000_000_000);
        assert_eq!(TAI_OFFSET_TAI_NS, 0);
        assert_eq!(GLONASS_UTC_EPOCH_SHIFT_NS, 757_371_600_000_000_000);
    }

    #[test]
    fn test_matrix_counts_are_correct() {
        let m = ConversionMatrix::new();
        // 6×6 матрица − 6 диагональных элементов = 30 внедиагональных ячеек
        // Fixed+Identity+EpochShift: симметрично, поэтому учитываем пары
        // GPS↔TAI(2) + GPS↔GAL(2) + GPS↔BDT(2) + GAL↔BDT(2) + GAL↔TAI(2) + BDT↔TAI(2) +
        // GLO↔UTC(2) = 14
        assert_eq!(m.path_count(false), 14, "14 fixed paths");
        // Оставшиеся 30 − 14 = 16 путей являются контекстуальными
        assert_eq!(m.path_count(true), 16, "16 contextual paths");
    }

    #[test]
    fn test_all_off_diagonal_cells_are_classified() {
        // Проверяем каждую пару (от, до) как фиксированную/идентичную/сдвиг эпохи или
        // контекстную.
        for &from in &ScaleId::ALL {
            for &to in &ScaleId::ALL {
                if from != to {
                    let kind = from.conversion_kind(to);
                    assert_ne!(
                        kind,
                        ConversionKind::SameScale,
                        "{:?}→{:?} should not be SameScale",
                        from,
                        to
                    );
                }
            }
        }
    }

    #[test]
    fn test_matrix_is_symmetric_in_kind_category() {
        // Для каждой пары классификация фиксированный против контекстуального должна
        // быть симметричной.
        for &from in &ScaleId::ALL {
            for &to in &ScaleId::ALL {
                if from != to {
                    let fwd_fixed = from.is_fixed(to);
                    let rev_fixed = to.is_fixed(from);
                    assert_eq!(
                        fwd_fixed, rev_fixed,
                        "{:?}↔{:?}: fixed classification must be symmetric",
                        from, to
                    );
                }
            }
        }
    }
}
