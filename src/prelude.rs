//! Prelude: удобные импорты для большинства сценариев использования.
//!
//! ```rust
//! use gnss_time::prelude::*;
//!
//! let gps = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();
//! let tai: Time<Tai> = gps.into_scale().unwrap();
//! ```

pub use crate::{
    // Трейты преобразования
    convert::{IntoScale, IntoScaleWith},
    // Високосные секунды
    leap::LeapSeconds,
    // Шкалы времени
    scale::{Beidou, Galileo, Glonass, Gps, Tai, Utc},
    // Базовые типы
    Duration,
    GnssTimeError,
    Time,
};
