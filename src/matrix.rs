//! # Матрица конверсий: полный граф поддерживаемых преобразований
//!
//! Этот модуль документирует и проверяет **полную матрицу** допустимых
//! конверсий между шкалами времени, а также предоставляет
//! [`crate::matrix::ConversionMatrix`]
//! - тип для runtime проверки совместимости шкал.
//!
//! ## Таблица оффсетов (источники: ICD-GLONASS, IS-GPS-200, OS-SIS-ICD Galileo, BDS-SIS-ICD)
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
//! Обозначения:
//! - `identity` - одинаковые наносекунды (GPS и Galileo делят TAI-offset 19с)
//! - `+N s` - фиксированное смещение, нет leap seconds
//! - `via UTC+LS` - требует явный [`LeapSecondsProvider`]
//! - `via LS` - требует явный [`LeapSecondsProvider`]
//! - `+757371600s` - константный сдвиг эпох GLONASS..UTC без leap seconds
//! - `no (ctx)` - невозможно без leap second контекста (contextual scale)
//!
//! ## Категории конверсий
//!
//! ### Фиксированные (без контекста)
//! Используют [`IntoScale`]:
//! - GLONASS <-> UTC
//! - GPS <-> TAI, GPS <-> Galileo <-> GPS <-> BeiDou
//! - Galileo <-> BeiDou, Galileo <-> TAI, BeiDou <-> TAI
//!
//! ### Константные (требует LeapSecondsProvider)
//! - GPS <-> UTC, GPS <-> GLONASS
//! - Galileo <-> UTC, Galileo <-> GLONASS
//! - BeiDou <-> UTC, BeiDou <-> GLONASS

use crate::{
    Beidou, Glonass, GnssTimeError, Gps, IntoScale, IntoScaleWith, LeapSecondsProvider, Tai, Time,
    Utc,
};

pub const TAI_OFFSET_GPS_NS: i64 = 19 * 1_000_000_000;
pub const TAI_OFFSET_GALILEO_NS: i64 = 19 * 1_000_000_000;
pub const TAI_OFFSET_BEIDOU_NS: i64 = 33 * 1_000_000_000;
pub const TAI_OFFSET_TAI_NS: i64 = 0;

pub const GLONASS_UTC_EPOCH_SHIFT_NS: i64 = 757_371_600 * 1_000_000_000;

/// Тип конверсии между двумя шкалами времени.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionKind {
    /// Фиксированное смещение - конверсии без контекста
    Fixed,

    /// Тождественное отображение (GPS <-> Galileo: одинаковые наносекунды)
    Identity,

    /// Константный сдвиг эпох без leap second контекста (GLONASS <-> UTC).
    EpochShift,

    /// Требует [`LeapSecondsProvider`]
    Contextual,

    /// Конверсия в себе (не нужна)
    SameScale,
}

/// Идентификатор шкалы времени (для runtime использования).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScaleId {
    Glonass,
    Gps,
    Galileo,
    Beidou,
    Tai,
    Utc,
}

/// Матрица конверсий: документирует и проверяет допустимые маршруты между всеми
/// шкалами времени.
///
/// # Пример
///
/// ```rust
/// use gnss_time::{ConversionMatrix, ScaleId};
///
/// // Проверка что GPS <-> Galileo - fixed конверсия
/// assert!(ScaleId::Gps.is_fixed(ScaleId::Galileo));
///
/// // Проверка что GPS <-> UTC требует leap seconds
/// assert!(ScaleId::Gps.needs_leap_seconds(ScaleId::Utc));
///
/// // Полная матрица 6х6
/// let matrix = ConversionMatrix::new();
///
/// assert_eq!(matrix.path_count(false), 14); // фиксированных путей
/// assert_eq!(matrix.path_count(true), 16); // контекстных путей
/// ```
pub struct ConversionMatrix;

#[derive(Debug)]
pub struct ConversionChain {
    pub gps: Time<Gps>,
    pub glonass: Time<Glonass>,
    pub utc: Time<Utc>,
    pub tai: Time<Tai>,
}

impl ScaleId {
    /// Все поддерживаемые шкалы.
    pub const ALL: [ScaleId; 6] = [
        ScaleId::Glonass,
        ScaleId::Gps,
        ScaleId::Galileo,
        ScaleId::Beidou,
        ScaleId::Tai,
        ScaleId::Utc,
    ];

    /// ASCII имя шкалы.
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
            // Galileo ↔ BeiDou: фиксированное (как и GPS <-> BeiDou)
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

    /// Возвращает `true` если конверсия `self -> target` не требует leap second
    /// контекста.
    pub const fn is_fixed(
        self,
        target: ScaleId,
    ) -> bool {
        matches!(
            self.conversion_kind(target),
            ConversionKind::Fixed | ConversionKind::Identity | ConversionKind::EpochShift
        )
    }

    /// Возвращает `true` если конверсия требует [`LeapSecondsProvider`].
    pub const fn needs_leap_seconds(
        self,
        target: ScaleId,
    ) -> bool {
        matches!(self.conversion_kind(target), ConversionKind::Contextual)
    }
}

impl ConversionMatrix {
    pub fn new() -> Self {
        ConversionMatrix
    }

    /// Кол-во путей заданного типа (fixed или contextual)
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

    /// Тип конверсии `from -> to`.
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

/// Выполнить конверсию GPS -> BeiDou -> GLONASS -> UTC -> TAI за один вызов.
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
