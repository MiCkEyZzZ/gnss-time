use gnss_time::{
    CivilDateTime, DurationParts, Gps, IntoScaleWith, LeapSeconds, Time, Utc,
    UTC_EPOCH_UNIX_OFFSET_S,
};

fn main() {
    println!("=== gnss-time: CivilDateTime / ISO 8601 demo ===\n");

    // ── Section 1: UTC epoch ──────────────────────────────────────────────────
    println!("── UTC epoch ────────────────────────────────────────────────");

    let utc_epoch = Time::<Utc>::EPOCH;
    let dt_epoch = utc_epoch.to_civil();

    println!("  Time::<Utc>::EPOCH.to_civil() = {dt_epoch}");

    assert_eq!(dt_epoch.to_string(), "1972-01-01T00:00:00.000000000Z");

    println!("  ✓ Matches 1972-01-01T00:00:00.000000000Z\n");

    // ── Section 2: GPS epoch as UTC ───────────────────────────────────────────
    println!("── GPS epoch (1980-01-06) as UTC ────────────────────────────");

    let utc_at_gps_epoch = Time::<Utc>::from_nanos(252_892_800_000_000_000);
    let dt_gps = utc_at_gps_epoch.to_civil();

    println!("  UTC at GPS epoch → {dt_gps}");

    assert_eq!(dt_gps.year, 1980);
    assert_eq!(dt_gps.month, 1);
    assert_eq!(dt_gps.day, 6);

    println!(
        "  ✓ year={}, month={}, day={}\n",
        dt_gps.year, dt_gps.month, dt_gps.day
    );

    // ── Section 3: Well-known timestamps ─────────────────────────────────────
    println!("── Well-known UTC timestamps ────────────────────────────────");

    let known: &[(&str, i64)] = &[
        ("1972-01-01 UTC epoch", 63_072_000),
        ("1980-01-06 GPS epoch", 315_964_800),
        ("1999-01-01 Y2K-1", 915_148_800),
        ("2009-01-01", 1_230_768_000),
        ("2017-01-01 last leap", 1_483_228_800),
        ("2024-01-01", 1_704_067_200),
    ];

    for (label, unix_s) in known {
        let utc = Time::<Utc>::from_unix_seconds(*unix_s).unwrap();
        let dt = utc.to_civil();

        println!("  {label:<25} → {dt}");
    }

    println!();

    // ── Section 4: Sub-second precision ──────────────────────────────────────
    println!("── Sub-second precision (nanoseconds) ───────────────────────");
    // 2024-01-15T12:34:56.123456789Z
    let day_ns: u64 = 19_007 * 86_400 * 1_000_000_000; // 2024-01-15 from UTC epoch
    let time_ns: u64 =
        12 * 3_600 * 1_000_000_000 + 34 * 60 * 1_000_000_000 + 56 * 1_000_000_000 + 123_456_789;
    let utc_sub = Time::<Utc>::from_nanos(day_ns + time_ns);
    let dt_sub = utc_sub.to_civil();

    println!("  {dt_sub}");

    assert_eq!(dt_sub.nanos, 123_456_789);

    println!("  ✓ Sub-second nanos = {}\n", dt_sub.nanos);

    // ── Section 5: Round-trip CivilDateTime → Time<Utc> → CivilDateTime ──────
    println!("── Round-trip Time<Utc> → CivilDateTime → Time<Utc> ─────────");

    let original = Time::<Utc>::from_nanos(1_700_000_000_500_000_000);
    let dt = original.to_civil();
    let back = dt.to_utc().unwrap();

    println!("  original nanos : {}", original.as_nanos());
    println!("  civil datetime : {dt}");
    println!("  round-trip     : {}", back.as_nanos());

    assert_eq!(original, back);

    println!("  ✓ Lossless round-trip\n");

    // ── Section 6: GPS → UTC → CivilDateTime ─────────────────────────────────
    println!("── GPS → UTC → CivilDateTime ────────────────────────────────");
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::from_week_tow(
        2243,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();
    let utc_from_gps: Time<Utc> = gps.into_scale_with(ls).unwrap();
    let dt_gps2 = utc_from_gps.to_civil();

    println!("  GPS {} → UTC → {dt_gps2}", gps);
    println!(
        "  GPS−UTC = 18 s in 2023: UTC seconds = {}",
        utc_from_gps.as_unix_seconds() - UTC_EPOCH_UNIX_OFFSET_S
    );
    println!();

    // ── Section 7: CivilDateTime from fields → Time<Utc> ─────────────────────
    println!("── CivilDateTime from fields → Time<Utc> ────────────────────");
    let dt_manual = CivilDateTime {
        year: 2024,
        month: 6,
        day: 15,
        hour: 18,
        minute: 30,
        second: 0,
        nanos: 0,
    };
    let utc_manual = dt_manual.to_utc().unwrap();

    println!(
        "  CivilDateTime {{ 2024-06-15 18:30:00 }} → {} (nanos from UTC epoch)",
        utc_manual.as_nanos()
    );
    println!("  as_unix_seconds = {}", utc_manual.as_unix_seconds());

    // Verify round-trip
    let dt_back = utc_manual.to_civil();

    assert_eq!(dt_back.year, 2024);
    assert_eq!(dt_back.month, 6);
    assert_eq!(dt_back.day, 15);
    assert_eq!(dt_back.hour, 18);
    assert_eq!(dt_back.minute, 30);

    println!("  ✓ Fields preserved through round-trip\n");

    // ── Section 8: Before UTC epoch → error ──────────────────────────────────
    println!("── Date before UTC epoch (1972-01-01) → error ───────────────");

    let dt_1970 = CivilDateTime {
        year: 1970,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0,
        nanos: 0,
    };
    let result = dt_1970.to_utc();

    println!("  CivilDateTime {{ 1970-01-01 }} .to_utc() = {:?}", result);

    assert!(result.is_err());

    println!("  ✓ Returns Err as expected\n");
    println!("=== All assertions passed ✓ ===");
}
