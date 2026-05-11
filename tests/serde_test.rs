#![cfg(feature = "serde")]

use gnss_time::{
    Beidou, Duration, DurationParts, Galileo, Glonass, Gps, LeapSeconds, Tai, Time, Utc,
};

#[test]
fn test_gps_postcard_roundtrip_epoch() {
    let t = Time::<Gps>::EPOCH;
    let bytes = postcard::to_allocvec(&t).unwrap();
    let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(t, back);
}

#[test]
fn test_gps_postcard_roundtrip_typical_timestamp() {
    // GPS time ≈ 2023-01-01: week 2243, TOW = 0
    let t = Time::<Gps>::from_nanos(1_356_566_418_000_000_000);
    let bytes = postcard::to_allocvec(&t).unwrap();
    let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(t, back);
}

#[test]
fn test_gps_postcard_roundtrip_with_sub_second() {
    let t = Time::<Gps>::from_nanos(1_356_566_418_123_456_789);
    let bytes = postcard::to_allocvec(&t).unwrap();
    let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(t, back);
}

#[test]
fn test_gps_postcard_roundtrip_max() {
    let t = Time::<Gps>::MAX;
    let bytes = postcard::to_allocvec(&t).unwrap();
    let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(t, back);
}

#[test]
fn test_utc_postcard_roundtrip() {
    let t = Time::<Utc>::from_nanos(1_514_764_800_000_000_000);
    let bytes = postcard::to_allocvec(&t).unwrap();
    let back: Time<Utc> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(t, back);
}

#[test]
fn test_tai_postcard_roundtrip() {
    let t = Time::<Tai>::from_seconds(100_000_000);
    let bytes = postcard::to_allocvec(&t).unwrap();
    let back: Time<Tai> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(t, back);
}

#[test]
fn test_galileo_postcard_roundtrip() {
    let t = Time::<Galileo>::from_nanos(999_999_999_999);
    let bytes = postcard::to_allocvec(&t).unwrap();
    let back: Time<Galileo> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(t, back);
}

#[test]
fn test_beidou_postcard_roundtrip() {
    let t = Time::<Beidou>::EPOCH;
    let bytes = postcard::to_allocvec(&t).unwrap();
    let back: Time<Beidou> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(t, back);
}

#[test]
fn test_glonass_postcard_roundtrip() {
    let t = Time::<Glonass>::from_nanos(42_000_000_000_000);
    let bytes = postcard::to_allocvec(&t).unwrap();
    let back: Time<Glonass> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(t, back);
}

// postcard uses ULEB-128 (unsigned LEB-128) for u64, so the encoded size
// depends on the magnitude:
//
//   value < 2^7   (128)          → 1 byte
//   value < 2^14  (16_384)       → 2 bytes
//   value < 2^21  (2_097_152)    → 3 bytes
//   value < 2^28                 → 4 bytes
//   value < 2^35                 → 5 bytes
//   value < 2^42                 → 6 bytes
//   value < 2^49                 → 7 bytes
//   value < 2^56                 → 8 bytes
//   value < 2^63                 → 9 bytes
//   value <= u64::MAX            → 10 bytes

#[test]
fn test_postcard_epoch_encodes_as_1_byte() {
    // EPOCH = 0, ULEB-128(0) = [0x00] → 1 byte
    let bytes = postcard::to_allocvec(&Time::<Gps>::EPOCH).unwrap();

    assert_eq!(bytes.len(), 1, "EPOCH (0) should encode as 1 byte");
    assert_eq!(bytes[0], 0x00);
}

#[test]
fn test_postcard_max_encodes_as_10_bytes() {
    // u64::MAX = 0xFFFF_FFFF_FFFF_FFFF → 10 bytes in ULEB-128
    let bytes = postcard::to_allocvec(&Time::<Gps>::MAX).unwrap();

    assert_eq!(bytes.len(), 10, "u64::MAX should encode as 10 bytes");
}

#[test]
fn test_postcard_size_is_at_most_10_bytes() {
    // For any Time<S>, the compact encoding is at most 10 bytes
    let cases: &[u64] = &[
        0,
        1,
        127,
        128,
        604_800_000_000_000,       // 1 week
        1_356_566_418_000_000_000, // ~2023
        u64::MAX,
    ];

    for &nanos in cases {
        let t = Time::<Gps>::from_nanos(nanos);
        let bytes = postcard::to_allocvec(&t).unwrap();

        assert!(
            bytes.len() <= 10,
            "Time<Gps>({nanos}) encoded to {} bytes, expected ≤ 10",
            bytes.len()
        );
    }
}

#[test]
fn test_postcard_is_more_compact_than_json_for_large_values() {
    // For typical GPS timestamps (~2023), postcard should be much smaller
    let t = Time::<Gps>::from_nanos(1_356_566_418_000_000_000);
    let json_bytes = serde_json::to_vec(&t).unwrap();
    let postcard_bytes = postcard::to_allocvec(&t).unwrap();

    assert!(
        postcard_bytes.len() < json_bytes.len(),
        "postcard ({} B) should be smaller than JSON ({} B)",
        postcard_bytes.len(),
        json_bytes.len()
    );
}

#[test]
fn test_postcard_size_one_week_is_8_bytes() {
    // 1 week = 604_800_000_000_000 ns
    // In binary: fits in 50 bits → ULEB-128 needs ceil(50/7) = 8 bytes
    let t = Time::<Gps>::from_nanos(604_800_000_000_000);
    let bytes = postcard::to_allocvec(&t).unwrap();

    assert_eq!(bytes.len(), 8, "1-week timestamp should encode as 8 bytes");
}

#[test]
fn test_postcard_wire_format_epoch_is_single_zero_byte() {
    let bytes = postcard::to_allocvec(&Time::<Gps>::EPOCH).unwrap();

    assert_eq!(&bytes[..], &[0x00]);
}

#[test]
fn test_postcard_wire_format_small_value() {
    // 1 nanosecond = 0x01 in ULEB-128
    let t = Time::<Gps>::from_nanos(1);
    let bytes = postcard::to_allocvec(&t).unwrap();

    assert_eq!(&bytes[..], &[0x01]);
}

#[test]
fn test_postcard_wire_format_127_nanoseconds() {
    // 127 = 0x7F — largest single-byte ULEB-128 value
    let t = Time::<Gps>::from_nanos(127);
    let bytes = postcard::to_allocvec(&t).unwrap();

    assert_eq!(bytes.len(), 1);
    assert_eq!(bytes[0], 0x7F);
}

#[test]
fn test_postcard_wire_format_128_nanoseconds() {
    // 128 = 0x80 → ULEB-128: [0x80, 0x01] (2 bytes)
    let t = Time::<Gps>::from_nanos(128);
    let bytes = postcard::to_allocvec(&t).unwrap();

    assert_eq!(bytes.len(), 2);
    assert_eq!(&bytes[..], &[0x80, 0x01]);
}

#[test]
fn test_postcard_gps_and_utc_same_nanos_different_types() {
    // In compact mode there is no scale tag, so the same bytes deserialize
    // into different types (as intended — scale is in the Rust type system).
    let nanos: u64 = 1_000_000_000_000;
    let gps = Time::<Gps>::from_nanos(nanos);
    let utc = Time::<Utc>::from_nanos(nanos);

    let gps_bytes = postcard::to_allocvec(&gps).unwrap();
    let utc_bytes = postcard::to_allocvec(&utc).unwrap();

    // Same underlying bytes
    assert_eq!(gps_bytes, utc_bytes);

    // Deserialize each into its own type — both succeed
    let gps_back: Time<Gps> = postcard::from_bytes(&gps_bytes).unwrap();
    let utc_back: Time<Utc> = postcard::from_bytes(&utc_bytes).unwrap();

    assert_eq!(gps_back.as_nanos(), nanos);
    assert_eq!(utc_back.as_nanos(), nanos);
}

#[test]
fn test_duration_postcard_roundtrip_zero() {
    let d = Duration::ZERO;
    let bytes = postcard::to_allocvec(&d).unwrap();
    let back: Duration = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(d, back);
}

#[test]
fn test_duration_postcard_roundtrip_positive() {
    let d = Duration::from_seconds(3600);
    let bytes = postcard::to_allocvec(&d).unwrap();
    let back: Duration = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(d, back);
}

#[test]
fn test_duration_postcard_roundtrip_negative() {
    let d = Duration::from_seconds(-3600);
    let bytes = postcard::to_allocvec(&d).unwrap();
    let back: Duration = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(d, back);
}

#[test]
fn test_duration_postcard_roundtrip_max() {
    let d = Duration::MAX;
    let bytes = postcard::to_allocvec(&d).unwrap();
    let back: Duration = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(d, back);
}

#[test]
fn test_duration_postcard_roundtrip_min() {
    let d = Duration::MIN;
    let bytes = postcard::to_allocvec(&d).unwrap();
    let back: Duration = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(d, back);
}

#[test]
fn test_duration_parts_postcard_roundtrip_zero() {
    let p = DurationParts {
        seconds: 0,
        nanos: 0,
    };
    let bytes = postcard::to_allocvec(&p).unwrap();
    let back: DurationParts = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(p, back);
}

#[test]
fn test_duration_parts_postcard_roundtrip_typical() {
    let p = DurationParts {
        seconds: 86_400,
        nanos: 500_000_000,
    };
    let bytes = postcard::to_allocvec(&p).unwrap();
    let back: DurationParts = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(p, back);
}

#[test]
fn test_duration_parts_postcard_roundtrip_max_nanos() {
    let p = DurationParts {
        seconds: 604_799,
        nanos: 999_999_999,
    };
    let bytes = postcard::to_allocvec(&p).unwrap();
    let back: DurationParts = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(p, back);
}

// In embedded no_std environments without alloc, postcard can serialize into
// a heapless::Vec instead of std::vec::Vec. The wire format is identical.
// We verify this by manually checking the ULEB-128 encoding against known
// expected bytes.

#[test]
fn test_heapless_compatibility_epoch_encoding() {
    // EPOCH = 0 → ULEB-128 = [0x00]
    // This is what postcard would write to a heapless::Vec<u8, 16>
    let t = Time::<Gps>::EPOCH;
    let bytes = postcard::to_allocvec(&t).unwrap();

    // Verify the encoding matches what heapless would produce
    assert_eq!(bytes.len(), 1);
    assert_eq!(bytes[0], 0x00);
}

#[test]
fn test_heapless_buffer_size_16_sufficient_for_typical_gps() {
    // A 16-byte heapless buffer is sufficient for any Time<S> value
    // (max ULEB-128 u64 = 10 bytes)
    let t = Time::<Gps>::MAX;
    let bytes = postcard::to_allocvec(&t).unwrap();

    assert!(
        bytes.len() <= 16,
        "16-byte buffer is sufficient for any Time<S>"
    );
}

#[test]
fn test_heapless_buffer_size_for_duration_parts() {
    // DurationParts = [u64, u32] = max 10 + 5 = 15 bytes
    let _p = DurationParts {
        seconds: u64::MAX,
        nanos: 999_999_999,
    };
    // Note: this would fail DurationParts::new() validation, so we test
    // a valid large value instead
    let p = DurationParts {
        seconds: u64::MAX / 1_000_000_000,
        nanos: 999_999_999,
    };
    let bytes = postcard::to_allocvec(&p).unwrap();

    assert!(
        bytes.len() <= 16,
        "16-byte buffer is sufficient for DurationParts"
    );
}

#[test]
fn test_json_and_postcard_produce_same_timestamp() {
    let original = Time::<Gps>::from_nanos(1_356_566_418_123_456_789);

    // Serialize to JSON, deserialize back
    let json = serde_json::to_string(&original).unwrap();
    let from_json: Time<Gps> = serde_json::from_str(&json).unwrap();

    // Serialize to postcard, deserialize back
    let postcard_bytes = postcard::to_allocvec(&original).unwrap();
    let from_postcard: Time<Gps> = postcard::from_bytes(&postcard_bytes).unwrap();

    // Both must equal the original
    assert_eq!(from_json, original);
    assert_eq!(from_postcard, original);
    assert_eq!(from_json, from_postcard);
}

#[test]
fn test_json_and_postcard_for_all_scales() {
    macro_rules! check_both_formats {
        ($scale:ty) => {
            let t = Time::<$scale>::from_nanos(1_000_000_000_000);

            let json = serde_json::to_string(&t).unwrap();
            let from_json: Time<$scale> = serde_json::from_str(&json).unwrap();

            let bytes = postcard::to_allocvec(&t).unwrap();
            let from_postcard: Time<$scale> = postcard::from_bytes(&bytes).unwrap();

            assert_eq!(t, from_json);
            assert_eq!(t, from_postcard);
        };
    }

    check_both_formats!(Gps);
    check_both_formats!(Utc);
    check_both_formats!(Tai);
    check_both_formats!(Galileo);
    check_both_formats!(Beidou);
    check_both_formats!(Glonass);
}

#[test]
fn test_gps_utc_convert_then_postcard_roundtrip() {
    let ls = LeapSeconds::builtin();

    let gps_original = Time::<Gps>::from_week_tow(
        2243,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    // Convert GPS → UTC
    let utc = gps_original.to_utc_with(&ls).unwrap();

    // Serialize UTC to postcard
    let bytes = postcard::to_allocvec(&utc).unwrap();
    let utc_back: Time<Utc> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(utc, utc_back);

    // Convert back UTC → GPS
    let gps_back = utc_back.to_gps_with(&ls).unwrap();

    assert_eq!(gps_original, gps_back);
}

#[test]
fn test_unix_time_postcard_roundtrip() {
    // Typical 2024 Unix timestamp
    let unix_s: i64 = 1_704_067_200;
    let utc = Time::<Utc>::from_unix_seconds(unix_s).unwrap();

    let bytes = postcard::to_allocvec(&utc).unwrap();
    let back: Time<Utc> = postcard::from_bytes(&bytes).unwrap();

    assert_eq!(utc, back);
    assert_eq!(back.as_unix_seconds(), unix_s);
}
