use core::str::FromStr;

use gnss_time::{Duration, Glonass, GnssTimeError, Gps, Time, Utc};

fn main() {
    println!("=== gnss-time: parsing strings ===\n");
    // ── Section 1: Time<Gps> ──────────────────────────────────────────────────
    println!("── Time<Gps>: \"GPS <week>:<tow_seconds>.<millis>\" ──────────");

    let gps: Time<Gps> = "GPS 2345:432000.000".parse().unwrap();

    println!(
        "  \"GPS 2345:432000.000\" → week={}, tow={}s",
        gps.week(),
        gps.tow_seconds()
    );

    assert_eq!(gps.week(), 2345);
    assert_eq!(gps.tow_seconds(), 432_000);

    // Round-trip through Display (millisecond-aligned values are exact)
    let s = gps.to_string();
    let back: Time<Gps> = s.parse().unwrap();

    assert_eq!(gps, back);

    println!("  round-trip via Display: \"{s}\" → equal ✓\n");
    // ── Section 2: Time<Glonass> ──────────────────────────────────────────────
    println!("── Time<Glonass>: \"GLO <day>:<tod_seconds>.<millis>\" ───────");

    let glo: Time<Glonass> = "GLO 10512:43200.000".parse().unwrap();

    println!(
        "  \"GLO 10512:43200.000\" → day={}, tod={}s",
        glo.day(),
        glo.tod_seconds()
    );

    assert_eq!(glo.day(), 10512);
    assert_eq!(glo.tod_seconds(), 43_200);

    let s = glo.to_string();
    let back: Time<Glonass> = s.parse().unwrap();

    assert_eq!(glo, back);

    println!("  round-trip via Display: \"{s}\" → equal ✓\n");
    // ── Section 3: Time<Utc> — ISO 8601 (full nanosecond precision) ──────────
    println!("── Time<Utc>: parsing ISO 8601 / RFC 3339 into Time<Utc> ────");

    let utc: Time<Utc> = "2024-01-15T12:34:56.123456789Z".parse().unwrap();
    let dt = utc.to_civil();

    println!(
        "  \"2024-01-15T12:34:56.123456789Z\" → {:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09}",
        dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second, dt.nanos
    );

    assert_eq!(dt.year, 2024);
    assert_eq!(dt.nanos, 123_456_789);

    // Exact round-trip via CivilDateTime's Display (NOT Time<Utc>'s own Display —
    // see the doc comment on `impl FromStr for Time<Utc>` for why).
    let original = Time::<Utc>::from_nanos(1_234_567_890_123_456_789);
    let s = original.to_civil().to_string();
    let back: Time<Utc> = s.parse().unwrap();

    assert_eq!(original, back);

    println!("  exact round-trip via to_civil().to_string(): \"{s}\" ✓\n");

    // UTC epoch
    let epoch: Time<Utc> = "1972-01-01T00:00:00.000000000Z".parse().unwrap();

    assert_eq!(epoch, Time::<Utc>::EPOCH);

    println!("  \"1972-01-01T00:00:00.000000000Z\" → Time::<Utc>::EPOCH ✓\n");

    // ── Section 4: Duration ───────────────────────────────────────────────────
    println!("── Duration: \"<seconds>s <nanos>ns\" ─────────────────────────");

    let d: Duration = "1s 500000000ns".parse().unwrap();

    println!("  \"1s 500000000ns\" → {} ns", d.as_nanos());

    assert_eq!(d.as_nanos(), 1_500_000_000);

    let neg: Duration = "-1s 500000000ns".parse().unwrap();

    println!("  \"-1s 500000000ns\" → {} ns", neg.as_nanos());

    assert_eq!(neg.as_nanos(), -1_500_000_000);

    let s = d.to_string();
    let back: Duration = s.parse().unwrap();

    assert_eq!(d, back);

    println!("  round-trip: \"{s}\" → equal ✓\n");

    // Errors fall into three classes:
    //   ParseError   — the string is structurally malformed
    //   InvalidInput — the string parses, but a field is out of its valid domain
    //   Overflow     — the string parses, the field is valid, but the value
    //                  cannot be represented in the target type
    // ── Section 5: Error handling ─────────────────────────────────────────────
    println!("── Error handling ────────────────────────────────────────────");

    let bad_prefix = Time::<Gps>::from_str("2345:432000.000");

    println!("  missing prefix          → {bad_prefix:?}");

    assert!(matches!(bad_prefix, Err(GnssTimeError::ParseError(_))));

    let bad_fraction = Time::<Gps>::from_str("GPS 2345:432000.0");

    println!("  wrong fraction width    → {bad_fraction:?}");

    assert!(matches!(bad_fraction, Err(GnssTimeError::ParseError(_))));

    let out_of_range = Time::<Gps>::from_str("GPS 0:604800.000"); // tow >= 604_800

    println!("  out-of-range tow        → {out_of_range:?}");

    assert!(matches!(out_of_range, Err(GnssTimeError::InvalidInput(_))));

    let before_epoch = Time::<Utc>::from_str("1970-01-01T00:00:00.000000000Z");

    println!("  before UTC epoch        → {before_epoch:?}");

    assert!(matches!(before_epoch, Err(GnssTimeError::Overflow)));

    let bad_duration = Duration::from_str("1s500000000ns"); // missing space

    println!("  Duration missing space  → {bad_duration:?}");

    assert!(matches!(bad_duration, Err(GnssTimeError::ParseError(_))));

    println!();
    // ── Section 6: Reading a config-style list of timestamps ─────────────────
    println!("── Config-file-style parsing ─────────────────────────────────");

    let config_lines = [
        "GPS 2200:000000.000",
        "GPS 2243:432000.500",
        "GPS 2300:100000.000",
    ];

    for line in config_lines {
        match Time::<Gps>::from_str(line) {
            Ok(t) => println!(
                "  {line:<24} → week={:>5}, tow={:>7}s",
                t.week(),
                t.tow_seconds()
            ),
            Err(e) => println!("  {line:<24} → ERROR: {e}"),
        }
    }

    println!("\n=== All assertions passed ✓ ===");
}
