//! # Маркерные типы временных шкал
//!
//! Каждая GNSS-система работает в собственной шкале времени с фиксированным
//! соотношением относительно TAI (Международного атомного времени).
//!
//! ## Запечатанный (sealed) трейит
//!
//! [`TimeScale`] нельзя реализовать вне этого crate — паттерн sealed
//! предотвращает случайное добавление пользовательских шкал времени.
//!
//! ## Форматы отображения
//!
//! | Шкала   | Пример формата              |
//! |---------|-----------------------------|
//! | GPS     | `"GPS 2345:432000.000"`     |
//! | GLONASS | `"GLO 10512:43200.000"`     |
//! | Galileo | `"GAL 1303:432000.000"`     |
//! | BeiDou  | `"BDT 960:432000.000"`      |
//! | TAI     | `"TAI +1000000000s 0ns"`    |
//! | UTC     | `"UTC +1000000000s 0ns"`    |

use crate::epoch::CivilDate;

// sealed-паттерн — запрещает внешние реализации
mod private {
    pub trait Sealed {}
}

macro_rules! define_scale {
    (
        $(#[$meta:meta])*
        $name:ident,
        display  = $display:literal,
        offset   = $offset:expr,
        epoch    = $epoch:expr,
        style    = $style:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
        pub struct $name;

        impl private::Sealed for $name {}

        impl TimeScale for $name {
            const NAME:          &'static str  = $display;
            const OFFSET_TO_TAI: OffsetToTai   = $offset;
            const EPOCH_CIVIL:   CivilDate     = $epoch;
            const DISPLAY_STYLE: DisplayStyle  = $style;
        }
    };
}

pub(crate) const NANOS_PER_SECOND: i64 = 1_000_000_000;

/// Связь временной шкалы с TAI.
///
/// Контракт (строгий):
///     T_tai = T_self + offset
///
/// Это должно быть согласовано для всех шкал.
/// Нарушение ломает межшкальные преобразования.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetToTai {
    /// Фиксированное смещение (не требует leap seconds)
    Fixed(i64),

    /// Зависит от внешнего контекста (UTC, GLONASS)
    Contextual,
}

/// Управляет тем, как [`Time<S>`] форматируется через [`Display`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayStyle {
    /// `"NAME WWW:SSSSSS.mm"` - неделя : время недели (GPS, Galileo, BeiDou)
    ///
    /// TOW seconds fields is always zero-padded to **6 digits** (max 604 799 s)
    WeekTow,

    /// `"NAME DDDDD:SSSSS.mmm"` - день : время суток (GLONASS)
    ///
    /// TOD seconds field is always zero-padded to **5 digits** (max 86 399 s)
    DayTod,

    /// `"NAME +Ss Nns"` - простой формат наносекунд для (TAI, UTC)
    Simple,
}

/// Маркерный трейит для GNSS / атомных шкал времени.
///
/// Этот трейит является **sealed** и не может быть реализован вне crate.
///
/// Каждая шкала определяет:
/// - [`NAME`] — короткое имя
/// - [`OFFSET_TO_TAI`] — преобразование в TAI
pub trait TimeScale: private::Sealed + Copy + Clone + Eq + PartialEq + core::fmt::Debug {
    /// Короткое имя шкалы (ASCII) используется в Display/debug
    const NAME: &'static str;

    /// Смещение относительно TAI:
    ///
    /// STRICT CONTRACT:
    ///     T_tai = T_self + offset
    ///
    /// Для контекстных шкал (UTC, GLONASS)
    /// требуется учёт leap seconds.
    const OFFSET_TO_TAI: OffsetToTai;

    /// Календарная дата эпохи шкалы
    /// (где `Time<S>::EPOCH == 0 ns`)
    const EPOCH_CIVIL: CivilDate;

    /// Формат отображения времени
    const DISPLAY_STYLE: DisplayStyle;
}

define_scale!(
    /// GLONASS — российская система времени (UTC(SU) + 3 часа)
    ///
    /// - Эпоха: 1996-01-01 00:00:00 UTC(SU)
    /// - Работает относительно UTC(SU)
    /// - Требует учёта високосных секунд
    /// - Формат: `"GLO 10512:43200.000"`
    Glonass,
    display = "GLO",
    offset = OffsetToTai::Contextual,
    epoch   = CivilDate::new(1996, 1, 1),
    style   = DisplayStyle::DayTod
);

define_scale!(
    /// GPS — американская система позиционирования
    ///
    /// - Эпоха: 1980-01-06 UTC
    /// - GPS = TAI − 19 секунд
    /// - Без leap seconds (фиксированное смещение)
    /// - Формат: `"GPS 2345:432000.000"`
    Gps,
    display = "GPS",
    offset  = OffsetToTai::Fixed(19 * NANOS_PER_SECOND),
    epoch   = CivilDate::new(1980, 1, 6),
    style   = DisplayStyle::WeekTow
);

define_scale!(
    /// Galileo — европейская система навигации (GST)
    ///
    /// - Эпоха: 1999-08-22 UTC
    /// - Совпадает по смещению с GPS (TAI − 19 s)
    /// - Одновременные значения = один и тот же момент времени
    /// - Формат: `"GAL 1303:432000.000"`
    Galileo,
    display = "GAL",
    offset = OffsetToTai::Fixed(19 * NANOS_PER_SECOND),
    epoch = CivilDate::new(1999, 8, 22),
    style   = DisplayStyle::WeekTow
);

define_scale!(
    /// BeiDou — китайская навигационная система (BDT)
    ///
    /// - Эпоха: 2006-01-01 UTC
    /// - BDT = TAI − 33 секунды
    /// - BDT = GPS − 14 секунд
    /// - Формат: `"BDT 960:432000.000"`
    Beidou,
    display = "BDT",
    offset = OffsetToTai::Fixed(33 * NANOS_PER_SECOND),
    epoch = CivilDate::new(2006, 1, 1),
    style = DisplayStyle::WeekTow
);

define_scale!(
    /// TAI — Международное атомное время
    ///
    /// - Эпоха: 1958-01-01
    /// - Базовая шкала для всех преобразований
    /// - TAI = TAI + 0
    /// - Формат: `"TAI +Ss Nns"`
    ///
    /// # Важно
    ///
    /// Внутри crate TAI используется как pivot для конверсий,
    /// а не как абсолютная шкала от 1958 года (это планируется отдельно).
    Tai,
    display = "TAI",
    offset = OffsetToTai::Fixed(0),
    epoch = CivilDate::new(1958, 1, 1),
    style = DisplayStyle::Simple
);

define_scale!(
    /// UTC — координированное всемирное время
    ///
    /// - UTC = TAI − LS(t)
    /// - Требует runtime leap-second таблицы
    /// - Формат: `"UTC +Ss Nns"`
    Utc,
    display = "UTC",
    offset = OffsetToTai::Contextual,
    epoch = CivilDate::new(1972, 1, 1),
    style = DisplayStyle::Simple
);

impl OffsetToTai {
    /// Возвращает фиксированное смещение в наносекундах
    #[inline(always)]
    pub const fn fixed(self) -> Option<i64> {
        match self {
            OffsetToTai::Fixed(v) => Some(v),
            OffsetToTai::Contextual => None,
        }
    }

    /// Возвращает `true` для шкал, требующих контекста времени выполнения (UTC,
    /// GLONASS).
    #[inline(always)]
    pub const fn is_contextual(self) -> bool {
        matches!(self, OffsetToTai::Contextual)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, mem::size_of};

    use super::*;

    #[test]
    fn test_name_are_correct() {
        assert_eq!(Glonass::NAME, "GLO");
        assert_eq!(Gps::NAME, "GPS");
        assert_eq!(Galileo::NAME, "GAL");
        assert_eq!(Beidou::NAME, "BDT");
        assert_eq!(Tai::NAME, "TAI");
        assert_eq!(Utc::NAME, "UTC");
    }

    #[test]
    fn test_fixed_offsets() {
        assert_eq!(
            Gps::OFFSET_TO_TAI,
            OffsetToTai::Fixed(19 * NANOS_PER_SECOND)
        );
        assert_eq!(
            Galileo::OFFSET_TO_TAI,
            OffsetToTai::Fixed(19 * NANOS_PER_SECOND)
        );
        assert_eq!(
            Beidou::OFFSET_TO_TAI,
            OffsetToTai::Fixed(33 * NANOS_PER_SECOND)
        );
        assert_eq!(Tai::OFFSET_TO_TAI, OffsetToTai::Fixed(0));
    }

    #[test]
    fn test_contextual_offsets() {
        assert_eq!(Utc::OFFSET_TO_TAI, OffsetToTai::Contextual);
        assert_eq!(Glonass::OFFSET_TO_TAI, OffsetToTai::Contextual);
    }

    #[test]
    fn test_scale_types_are_copy() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<Glonass>();
        assert_copy::<Gps>();
        assert_copy::<Galileo>();
        assert_copy::<Beidou>();
        assert_copy::<Tai>();
        assert_copy::<Utc>();
    }

    #[test]
    fn test_gps_and_galileo_are_aligned() {
        // Same TAI offset → simultaneous instants
        assert_eq!(Gps::OFFSET_TO_TAI, Galileo::OFFSET_TO_TAI);
    }

    #[test]
    fn test_tai_invariant_is_valid() {
        assert_eq!(Tai::OFFSET_TO_TAI, OffsetToTai::Fixed(0));
        assert!(Tai::OFFSET_TO_TAI.fixed().unwrap() == 0);
    }

    #[test]
    fn test_names_are_unique() {
        let names = [
            Gps::NAME,
            Glonass::NAME,
            Galileo::NAME,
            Beidou::NAME,
            Tai::NAME,
            Utc::NAME,
        ];
        let set: HashSet<_> = names.iter().collect();
        assert_eq!(set.len(), names.len());
    }

    #[test]
    fn test_fixed_scales_are_really_fixed() {
        let fixed_scales = [
            Gps::OFFSET_TO_TAI,
            Galileo::OFFSET_TO_TAI,
            Beidou::OFFSET_TO_TAI,
            Tai::OFFSET_TO_TAI,
        ];

        for scale in fixed_scales {
            assert!(scale.fixed().is_some(), "Expected Fixed, got Contextual");
        }
    }

    #[test]
    fn test_contextual_only_where_expected() {
        assert!(Utc::OFFSET_TO_TAI.is_contextual());
        assert!(Glonass::OFFSET_TO_TAI.is_contextual());
    }

    #[test]
    fn test_scale_is_zero_sized() {
        assert_eq!(size_of::<Glonass>(), 0);
        assert_eq!(size_of::<Gps>(), 0);
        assert_eq!(size_of::<Galileo>(), 0);
        assert_eq!(size_of::<Beidou>(), 0);
        assert_eq!(size_of::<Tai>(), 0);
        assert_eq!(size_of::<Utc>(), 0);
    }

    #[test]
    fn test_display_styles() {
        assert_eq!(Gps::DISPLAY_STYLE, DisplayStyle::WeekTow);
        assert_eq!(Glonass::DISPLAY_STYLE, DisplayStyle::DayTod);
        assert_eq!(Galileo::DISPLAY_STYLE, DisplayStyle::WeekTow);
        assert_eq!(Beidou::DISPLAY_STYLE, DisplayStyle::WeekTow);
        assert_eq!(Tai::DISPLAY_STYLE, DisplayStyle::Simple);
        assert_eq!(Utc::DISPLAY_STYLE, DisplayStyle::Simple);
    }

    #[test]
    fn test_epoch_civil_dates() {
        assert_eq!(Gps::EPOCH_CIVIL.year, 1980);
        assert_eq!(Glonass::EPOCH_CIVIL.year, 1996);
        assert_eq!(Galileo::EPOCH_CIVIL.year, 1999);
        assert_eq!(Beidou::EPOCH_CIVIL.year, 2006);
        assert_eq!(Tai::EPOCH_CIVIL.year, 1958);
    }

    #[test]
    fn test_tai_invariant() {
        assert_eq!(Tai::OFFSET_TO_TAI, OffsetToTai::Fixed(0));
    }

    #[test]
    fn test_contract_all_scales() {
        fn check<T: TimeScale>() {
            match T::OFFSET_TO_TAI {
                OffsetToTai::Fixed(0) => assert_eq!(T::NAME, "TAI"),
                OffsetToTai::Fixed(_) => { /* GPS, GAL, BDT */ }
                OffsetToTai::Contextual => {
                    assert!(T::NAME == "UTC" || T::NAME == "GLO")
                }
            }
        }
        check::<Gps>();
        check::<Glonass>();
        check::<Galileo>();
        check::<Beidou>();
        check::<Tai>();
        check::<Utc>();
    }
}
