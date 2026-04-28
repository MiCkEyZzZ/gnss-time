use gnss_time::{scale::Gps, Duration, GnssTimeError, Time};

fn main() {
    println!("=== Embedded-safe arithmetic demo ===\n");

    // Насыщение — ограничение на MAX/EPOCH, никогда не паникует
    let t = Time::<Gps>::MAX;
    let safe = t.saturating_add(Duration::from_seconds(1)); // никогда не паникуем

    assert_eq!(t, safe);

    println!("saturating_add: MAX + 1s = MAX (clamped) ✓");

    let epoch = Time::<Gps>::EPOCH;
    let safe2 = epoch.saturating_sub_duration(Duration::from_nanos(1));

    assert_eq!(safe2, Time::<Gps>::EPOCH);

    println!("saturating_sub: EPOCH - 1ns = EPOCH (clamped) ✓");

    // Проверено — возвращает None при переполнении
    let t2 = Time::<Gps>::from_seconds(1_000_000);
    match t2.checked_add(Duration::from_seconds(500)) {
        Some(result) => println!("checked_add: {} + 500s = {} ✓", t2, result),
        None => println!("checked_add: overflow!"),
    }

    match Time::<Gps>::MAX.checked_add(Duration::ONE_NANOSECOND) {
        Some(_) => unreachable!(),
        None => println!("checked_add: MAX + 1ns = None (overflow detected) ✓"),
    }

    // Fallible — возвращает GnssTimeError::Overflow
    match Time::<Gps>::MAX.try_add(Duration::from_seconds(1)) {
        Ok(_) => unreachable!(),
        Err(GnssTimeError::Overflow) => println!("try_add: MAX + 1s = Overflow ✓"),
        Err(e) => panic!("unexpected error: {e}"),
    }

    // Статический инициализатор (безопасен в no_std)
    static REFERENCE: Time<Gps> = Time::<Gps>::EPOCH;
    static WINDOW: Duration = Duration::from_seconds(30);

    println!(
        "\nStatic initializer: reference={REFERENCE}, window={}s ✓",
        WINDOW.as_seconds()
    );

    // Обычная арифметика — вызывает панику при переполнении, нормально, если
    // известны границы
    let gps = Time::<Gps>::from_week_tow(2345, 0.0).unwrap();
    let later = gps + Duration::from_seconds(3600); // Безопасно — не переполнится

    println!("\nPanicking add (safe): {gps} + 1h = {later} ✓");

    println!("\nAll safe arithmetic demos passed.");
}
