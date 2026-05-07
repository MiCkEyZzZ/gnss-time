use gnss_time::{
    Gps, IntoScaleWith, LeapSeconds, Time, Utc, UTC_EPOCH_UNIX_OFFSET_NS, UTC_EPOCH_UNIX_OFFSET_S,
};

fn main() {
    println!("=== gnss-time: Unix time interoperability ===\n");

    // ─────────────────────────────────────────────────────────────────────────
    // Section 1: Epoch constants
    // ─────────────────────────────────────────────────────────────────────────
    println!("Epoch constants");
    println!(
        "  UTC_EPOCH_UNIX_OFFSET_S  = {:>15} s   (730 days; 1970→1972)",
        UTC_EPOCH_UNIX_OFFSET_S
    );
    println!(
        "  UTC_EPOCH_UNIX_OFFSET_NS = {:>15} ns  (same in nanoseconds)",
        UTC_EPOCH_UNIX_OFFSET_NS
    );
    println!();

    // ─────────────────────────────────────────────────────────────────────────
    // Section 2: Unix epoch (1970-01-01) is BEFORE the UTC epoch
    // ─────────────────────────────────────────────────────────────────────────
    println!("Unix epoch (1970-01-01) is before the UTC epoch (1972-01-01)");

    let result = Time::<Utc>::from_unix_seconds(0);

    println!("  Time::<Utc>::from_unix_seconds(0) -> {:?}", result);

    assert!(result.is_err(), "unix=0 must fail: before UTC epoch");

    let result_ns = Time::<Utc>::from_unix_nanos(0);

    println!("  Time::<Utc>::from_unix_nanos(0) -> {:?}", result_ns);

    assert!(result_ns.is_err(), "unix_ns=0 must fail");

    println!();

    // ─────────────────────────────────────────────────────────────────────────
    // Section 3: UTC epoch from Unix seconds
    // ─────────────────────────────────────────────────────────────────────────
    println!("UTC epoch from Unix time");

    let utc_epoch_via_unix = Time::<Utc>::from_unix_seconds(UTC_EPOCH_UNIX_OFFSET_S).unwrap();

    println!(
        "  from_unix_seconds({}) -> {} (UTC epoch = {})",
        UTC_EPOCH_UNIX_OFFSET_S,
        utc_epoch_via_unix,
        Time::<Utc>::EPOCH
    );

    assert_eq!(utc_epoch_via_unix, Time::<Utc>::EPOCH);

    let utc_epoch_via_unix_ns = Time::<Utc>::from_unix_nanos(UTC_EPOCH_UNIX_OFFSET_NS).unwrap();

    assert_eq!(utc_epoch_via_unix_ns, Time::<Utc>::EPOCH);

    println!(
        "  from_unix_nanos({}) -> {} ✓",
        UTC_EPOCH_UNIX_OFFSET_NS, utc_epoch_via_unix_ns
    );
    println!();

    // ─────────────────────────────────────────────────────────────────────────
    // Section 4: Round-trip Unix ↔ UTC (seconds)
    // ─────────────────────────────────────────────────────────────────────────
    println!("Round-trip Unix <-> Time<Utc> (seconds)");

    let test_unix_timestamps: &[i64] = &[
        63_072_000,    // 1972-01-01 = UTC epoch
        252_892_800,   // 1978-01-01
        315_964_800,   // 1980-01-06 = GPS epoch
        820_108_800,   // 1996-01-01 = GLONASS epoch (approx)
        1_000_000_000, // 2001-09-09 01:46:40
        1_577_836_800, // 2020-01-01 00:00:00
        1_700_000_000, // 2023-11-14 22:13:20
        1_704_067_200, // 2024-01-01 00:00:00
    ];

    for &unix_s in test_unix_timestamps {
        let utc = Time::<Utc>::from_unix_seconds(unix_s).unwrap();
        let back = utc.as_unix_seconds();
        assert_eq!(back, unix_s, "round-trip failed for unix_s={}", unix_s);
        println!("  unix={:>13} -> {} -> unix={}", unix_s, utc, back);
    }

    println!();

    // ─────────────────────────────────────────────────────────────────────────
    // Section 5: Round-trip Unix ↔ UTC (nanoseconds with sub-second precision)
    // ─────────────────────────────────────────────────────────────────────────
    println!("Round-trip Unix <-> Time<Utc> (nanoseconds)");

    let test_unix_nanos: &[i64] = &[
        UTC_EPOCH_UNIX_OFFSET_NS,     // UTC epoch exactly
        UTC_EPOCH_UNIX_OFFSET_NS + 1, // 1 ns after UTC epoch
        1_700_000_000_123_456_789,    // 2023 with sub-second
        1_704_067_200_500_000_000,    // 2024-01-01 + 0.5 s
    ];

    for &unix_ns in test_unix_nanos {
        let utc = Time::<Utc>::from_unix_nanos(unix_ns).unwrap();
        let back = utc.as_unix_nanos();

        assert_eq!(back, unix_ns, "round-trip failed for unix_ns={}", unix_ns);

        println!("  unix_ns={:<22} -> as_unix_nanos={}", unix_ns, back);
    }

    println!();

    // ─────────────────────────────────────────────────────────────────────────
    // Section 6: GPS ↔ Unix (via UTC + leap seconds)
    // ─────────────────────────────────────────────────────────────────────────
    println!("GPS <-> Unix (via UTC + leap seconds)");

    let ls = LeapSeconds::builtin();

    // GPS epoch in Unix time
    let gps_epoch_unix: i64 = 315_964_800;
    let gps = Time::<Gps>::from_unix_seconds(gps_epoch_unix, ls).unwrap();

    println!(
        "  from_unix_seconds({}) -> {} (expect GPS EPOCH)",
        gps_epoch_unix, gps
    );

    assert_eq!(gps, Time::<Gps>::EPOCH);

    // GPS epoch back to Unix
    let unix_back = Time::<Gps>::EPOCH.as_unix_seconds(ls).unwrap();

    println!("  GPS EPOCH.as_unix_seconds()        -> {}", unix_back);

    assert_eq!(unix_back, gps_epoch_unix);

    // Several well-known GPS ↔ Unix round-trips
    let test_gps_unix: &[i64] = &[
        315_964_800,   // 1980-01-06 GPS epoch (GPS-UTC=0)
        630_720_013,   // ~1990 (GPS-UTC=6)
        1_000_000_018, // ~2001 (GPS-UTC=13)
        1_577_836_818, // 2020-01-01 (GPS-UTC=18)
        1_672_531_218, // 2023-01-01 (GPS-UTC=18)
    ];

    println!();
    println!("  GPS <-> Unix round-trips:");

    for &unix_s in test_gps_unix {
        let gps_t = Time::<Gps>::from_unix_seconds(unix_s, ls).unwrap();
        let back = gps_t.as_unix_seconds(ls).unwrap();

        assert_eq!(back, unix_s, "round-trip failed for unix_s={}", unix_s);

        println!("    unix={:<13} -> {} -> unix={}", unix_s, gps_t, back);
    }

    println!();

    // ─────────────────────────────────────────────────────────────────────────
    // Section 7: GPS−UTC offset verification via Unix
    // ─────────────────────────────────────────────────────────────────────────
    println!("GPS − UTC offset verification");

    // After 2017-01-01 leap second: GPS − UTC = 18 s
    // Unix 2023-01-01 = 1_672_531_200
    let unix_2023: i64 = 1_672_531_200;
    let gps_epoch_unix: i64 = 315_964_800; // from epoch.rs
    let utc_2023 = Time::<Utc>::from_unix_seconds(unix_2023).unwrap();
    let gps_2023: Time<Gps> = utc_2023.into_scale_with(ls).unwrap();

    // GPS−UTC = gps.as_seconds() - (unix_2023 - gps_epoch_unix)
    let gps_utc_offset = (gps_2023.as_seconds() as i64) - (unix_2023 - gps_epoch_unix);

    println!(
        "  2023-01-01 UTC: unix={}, GPS={}, GPS−UTC offset = {} s (expected 18)",
        unix_2023, gps_2023, gps_utc_offset
    );

    assert_eq!(gps_utc_offset, 18, "GPS−UTC must be 18 in 2023");

    let _ = gps_utc_offset;

    println!();

    // ─────────────────────────────────────────────────────────────────────────
    // Section 8: Integration with std::time::SystemTime (feature = "std")
    // ─────────────────────────────────────────────────────────────────────────
    println!("std::time::SystemTime integration pattern");

    // Pattern for converting SystemTime to Time<Utc>:
    //
    // use std::time::{SystemTime, UNIX_EPOCH};
    //
    // let now = SystemTime::now();
    // let unix_ns = now.duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
    // let utc = Time::<Utc>::from_unix_nanos(unix_ns).unwrap();
    //
    // We simulate this with a fixed value to keep the example deterministic:
    let simulated_unix_ns: i64 = 1_700_000_000_000_000_000; // 2023-11-14
    let utc_from_system = Time::<Utc>::from_unix_nanos(simulated_unix_ns).unwrap();

    println!(
        "  Simulated SystemTime unix_ns={} -> {}",
        simulated_unix_ns, utc_from_system
    );
    println!(
        "  -> as Unix seconds: {}",
        utc_from_system.as_unix_seconds()
    );

    // Convert to GPS for GNSS processing
    let gps_from_system: Time<Gps> = utc_from_system.into_scale_with(ls).unwrap();

    println!("  -> as GPS time:     {}", gps_from_system);
    println!(
        "  -> GPS week:        {}, TOW: {} s",
        gps_from_system.week(),
        gps_from_system.tow_seconds()
    );
    println!();
    println!("=== All assertions passed ✓ ===");
}
