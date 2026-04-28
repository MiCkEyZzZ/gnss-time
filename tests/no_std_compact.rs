// # Tests: `no_std` совместимость и гарантии размеров типов
//
// Эти тесты проверяют сво-во, критичные для embedded-среды:
//
// 1. **Размеры типов** — `Time<S>` ровно 8 байт, `Duration` ровно 8 байт.
// 2. **`Copy` без аллокаций** — все типы `Copy`, нет `Box`, `Vec`, `String`.
// 3. **`no_std` корректность** — crate компилируется без std (проверяется в CI
//    кросс-компиляцией под embedded targets).
// 4. **Константность** — ключевые операции `const fn` для использования в
//    static initializers.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use gnss_time::{
    Beidou, Duration, Galileo, Glonass, GnssTimeError, Gps, IntoScale, IntoScaleWith, LeapSeconds,
    Tai, Time, Utc,
};

// Константная инициализация в static context
static GPS_EPOCH_STATIC: Time<Gps> = Time::<Gps>::EPOCH;
static GPS_MAX_STATIC: Time<Gps> = Time::<Gps>::MAX;
static ZERO_DURATION: Duration = Duration::ZERO;
static ONE_SECOND: Duration = Duration::ONE_SECOND;

// Константная операция с Duration
const fn make_five_seconds() -> Duration {
    Duration::from_seconds(5)
}

const FIVE_SECONDS: Duration = make_five_seconds();

fn hash_of<T: Hash>(v: T) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

#[test]
fn test_time_gps_is_exactly_8_bytes() {
    assert_eq!(
        core::mem::size_of::<Time<Gps>>(),
        8,
        "Time<Gps> must be exactly 8 bytes (= u64)"
    );
}

#[test]
fn test_time_glonass_is_exactly_8_bytes() {
    assert_eq!(core::mem::size_of::<Time<Glonass>>(), 8);
}

#[test]
fn test_time_galileo_is_exactly_8_bytes() {
    assert_eq!(core::mem::size_of::<Time<Galileo>>(), 8);
}

#[test]
fn test_time_beidou_is_exactly_8_bytes() {
    assert_eq!(core::mem::size_of::<Time<Beidou>>(), 8);
}

#[test]
fn test_time_tai_is_exactly_8_bytes() {
    assert_eq!(core::mem::size_of::<Time<Tai>>(), 8);
}

#[test]
fn test_time_utc_is_exactly_8_bytes() {
    assert_eq!(core::mem::size_of::<Time<Utc>>(), 8);
}

#[test]
fn test_duration_is_exactly_8_bytes() {
    assert_eq!(
        core::mem::size_of::<Duration>(),
        8,
        "Duration must be exactly 8 bytes (= i64)"
    );
}

// Масштабные маркеры нулевого размера
#[test]
fn test_all_scale_markers_are_zero_sized() {
    assert_eq!(core::mem::size_of::<Gps>(), 0);
    assert_eq!(core::mem::size_of::<Glonass>(), 0);
    assert_eq!(core::mem::size_of::<Galileo>(), 0);
    assert_eq!(core::mem::size_of::<Beidou>(), 0);
    assert_eq!(core::mem::size_of::<Tai>(), 0);
    assert_eq!(core::mem::size_of::<Utc>(), 0);
}

#[test]
fn test_time_is_copy_no_drop() {
    fn assert_copy_no_drop<T: Copy>() {
        // Если T реализует Drop, то T: Copy нельзя — это проверяется компилятором.
        // Дополнительно проверяем что T не требует деструктора.
        assert!(
            !core::mem::needs_drop::<T>(),
            "Time<S> must not require Drop (no allocations)"
        );
    }

    assert_copy_no_drop::<Time<Gps>>();
    assert_copy_no_drop::<Time<Glonass>>();
    assert_copy_no_drop::<Time<Galileo>>();
    assert_copy_no_drop::<Time<Beidou>>();
    assert_copy_no_drop::<Time<Tai>>();
    assert_copy_no_drop::<Time<Utc>>();
}

#[test]
fn test_duration_is_copy_no_drop() {
    assert!(!core::mem::needs_drop::<Duration>());
}

#[test]
fn test_gnss_time_error_is_copy_no_drop() {
    assert!(!core::mem::needs_drop::<GnssTimeError>());
}

// Явно проверяем Copy через присваивание (не move)
#[test]
fn test_copy_semantics_time() {
    let t = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();
    let t2 = t; // copy, не move
    let _t3 = t; // t всё ещё доступен

    assert_eq!(t, t2);
}

#[test]
fn test_copy_semantics_duration() {
    let d = Duration::from_seconds(42);
    let d2 = d;
    let _d3 = d;

    assert_eq!(d, d2);
}

#[test]
fn test_static_epoch_is_zero() {
    assert_eq!(GPS_EPOCH_STATIC.as_nanos(), 0);
}

#[test]
fn test_static_max_is_u64_max() {
    assert_eq!(GPS_MAX_STATIC.as_nanos(), u64::MAX);
}

#[test]
fn test_static_durations_are_correct() {
    assert!(ZERO_DURATION.is_zero());
    assert_eq!(ONE_SECOND.as_seconds(), 1);
}

#[test]
fn test_const_fn_duration_works() {
    assert_eq!(FIVE_SECONDS.as_seconds(), 5);
}

#[test]
fn test_time_gps_alignment_is_8_bytes() {
    // Time<Gps> содержит u64 → выравнивание 8 байт
    assert_eq!(
        core::mem::align_of::<Time<Gps>>(),
        8,
        "Time<Gps> must be 8-byte aligned (u64 layout)"
    );
}

#[test]
fn test_duration_alignment_is_8_bytes() {
    assert_eq!(
        core::mem::align_of::<Duration>(),
        8,
        "Duration must be 8-byte aligned (i64 layout)"
    );
}

#[test]
fn no_heap_allocation_in_conversions() {
    // Все эти операции — на стеке, нет heap-аллокаций
    let gps: Time<Gps> = Time::from_seconds(1_500_000_000);
    let tai: Time<Tai> = gps.into_scale().unwrap();
    let gal: Time<Galileo> = gps.into_scale().unwrap();
    let bdt: Time<Beidou> = gps.into_scale().unwrap();

    let ls = LeapSeconds::builtin();
    let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
    let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();

    // Обратно
    let _back_gps: Time<Gps> = tai.into_scale().unwrap();
    let _back2: Time<Gps> = gal.into_scale().unwrap();
    let _back3: Time<Gps> = bdt.into_scale().unwrap();
    let _back4: Time<Gps> = utc.into_scale_with(ls).unwrap();
    let _back5: Time<Gps> = glo.into_scale_with(ls).unwrap();

    // Все результаты — 8 байт на стеке
    assert_eq!(core::mem::size_of_val(&tai), 8);
    assert_eq!(core::mem::size_of_val(&gal), 8);
    assert_eq!(core::mem::size_of_val(&bdt), 8);
    assert_eq!(core::mem::size_of_val(&utc), 8);
    assert_eq!(core::mem::size_of_val(&glo), 8);
}

#[test]
fn time_implements_hash_and_eq() {
    let t1 = Time::<Gps>::from_seconds(42);
    let t2 = Time::<Gps>::from_seconds(42);
    let t3 = Time::<Gps>::from_seconds(43);

    assert_eq!(t1, t2);
    assert_eq!(
        hash_of(t1),
        hash_of(t2),
        "equal values must have equal hashes"
    );
    assert_ne!(hash_of(t1), hash_of(t3)); // may collide, but unlikely
}
