//! Serde support for `gnss-time` types
//!
//! Enabled with the `serde` feature flag:
//!
//! ```toml
//! [dependensies]
//! gnss-time = { version = "0.5", features = ["serde"] }
//! ```
//!
//! # Formats
//!
//! ### `Time<S>`
//!
//! **Human-readable** (JSON, TOML, YAML, ...):
//!
//! ```json
//! { "scale": "GPS", "nanos": 1356566418000000000 }
//! ```
//!
//! The `scale` field is validated during deserialization — deserializing a
//! JSON object with `"scale": "UTC"` into `Time<Gps>` returns an error.
//!
//! **Compact** (postcard, bincode, `MessagePack`, …):
//!
//! Raw `u64` nanoseconds. No scale tag is stored.
//!
//! ### `Duration`
//!
//! **Human-readable**: `{ "nanos": -7000000000 }`
//!
//! **Compact**: raw `i64` nanoseconds.
//!
//! ### `DurationParts`
//!
//! **Human-readable**: `{ "seconds": 5, "nanos": 500000000 }`
//!
//! **Compact**: 2-element tuple `[u64, u32]`.
//!
//! ## `no_std` compatibility
//!
//! All implementations use `serde`'s `no_std`-compatible API.
//!
//! ## Examples
//!
//! ```rust
//! # #[cfg(feature = "serde")] {
//! use gnss_time::{Duration, DurationParts, Gps, Time};
//!
//! // Time<Gps> — JSON round-trip
//! let gps = Time::<Gps>::from_seconds(1_356_566_418);
//! let json = serde_json::to_string(&gps).unwrap();
//! assert_eq!(json, r#"{"scale":"GPS","nanos":1356566418000000000}"#);
//! let back: Time<Gps> = serde_json::from_str(&json).unwrap();
//! assert_eq!(gps, back);
//!
//! // Duration — JSON round-trip
//! let d = Duration::from_seconds(-7);
//! let json = serde_json::to_string(&d).unwrap();
//! assert_eq!(json, r#"{"nanos":-7000000000}"#);
//! let back: Duration = serde_json::from_str(&json).unwrap();
//! assert_eq!(d, back);
//! # }
//! ```

use core::{fmt, marker::PhantomData};

use serde::{
    de::{self, Deserializer, MapAccess, SeqAccess, Visitor},
    ser::{SerializeStruct, SerializeTuple, Serializer},
    Deserialize, Serialize,
};

use crate::{Duration, DurationParts, GnssTimeError, Time, TimeScale};

const TIME_FIELDS: &[&str] = &["scale", "nanos"];
const DURATION_PARTS_FIELDS: &[&str] = &["seconds", "nanos"];

enum TimeField {
    Scale,
    Nanos,
}

enum DurationField {
    Nanos,
}

enum DurationPartsField {
    Seconds,
    Nanos,
}

struct TimeVisitor<S: TimeScale>(PhantomData<S>);

// Error helper that implements `fmt::Display` without allocation.
struct ScaleMismatch<'a> {
    expected: &'a str,
    got: &'a str,
}

struct DurationVisitor;

struct DurationPartsMapVisitor;

struct DurationPartsTupleVisitor;

impl<S: TimeScale> Serialize for Time<S> {
    fn serialize<Ser: Serializer>(
        &self,
        serializer: Ser,
    ) -> Result<Ser::Ok, Ser::Error> {
        if serializer.is_human_readable() {
            // JSON / TOML: { "scale": "GPS", "nanos": 12345678 }
            let mut s = serializer.serialize_struct("Time", 2)?;
            s.serialize_field("scale", S::NAME)?;
            s.serialize_field("nanos", &self.as_nanos())?;
            s.end()
        } else {
            // postcard / bincode: raw u64 nanoseconds
            serializer.serialize_u64(self.as_nanos())
        }
    }
}

impl<'de, S: TimeScale> Deserialize<'de> for Time<S> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_struct("Time", TIME_FIELDS, TimeVisitor::<S>(PhantomData))
        } else {
            let nanos = u64::deserialize(deserializer)?;
            Ok(Time::from_nanos(nanos))
        }
    }
}

impl<'de, S: TimeScale> Visitor<'de> for TimeVisitor<S> {
    type Value = Time<S>;

    fn expecting(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, r#"a map {{ "scale": "{}", "nanos": u64 }}"#, S::NAME)
    }

    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> Result<Self::Value, A::Error> {
        let mut nanos: Option<u64> = None;

        while let Some(key) = map.next_key::<TimeField>()? {
            match key {
                TimeField::Scale => {
                    let value: &str = map.next_value()?;
                    if value != S::NAME {
                        return Err(de::Error::custom(ScaleMismatch {
                            expected: S::NAME,
                            got: value,
                        }));
                    }
                }
                TimeField::Nanos => {
                    nanos = Some(map.next_value()?);
                }
            }
        }

        let nanos = nanos.ok_or_else(|| de::Error::missing_field("nanos"))?;
        Ok(Time::from_nanos(nanos))
    }
}

impl<'de> Deserialize<'de> for TimeField {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TimeFieldVisitor;

        impl Visitor<'_> for TimeFieldVisitor {
            type Value = TimeField;

            fn expecting(
                &self,
                f: &mut fmt::Formatter<'_>,
            ) -> fmt::Result {
                f.write_str("`scale` or `nanos`")
            }

            fn visit_str<E: de::Error>(
                self,
                v: &str,
            ) -> Result<TimeField, E> {
                match v {
                    "scale" => Ok(TimeField::Scale),
                    "nanos" => Ok(TimeField::Nanos),
                    other => Err(de::Error::unknown_field(other, TIME_FIELDS)),
                }
            }
        }

        deserializer.deserialize_identifier(TimeFieldVisitor)
    }
}

impl Serialize for Duration {
    fn serialize<Ser: Serializer>(
        &self,
        serializer: Ser,
    ) -> Result<Ser::Ok, Ser::Error> {
        if serializer.is_human_readable() {
            // { "nanos": -7000000000 }
            let mut s = serializer.serialize_struct("Duration", 1)?;

            s.serialize_field("nanos", &self.as_nanos())?;
            s.end()
        } else {
            serializer.serialize_i64(self.as_nanos())
        }
    }
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_struct("Duration", &["nanos"], DurationVisitor)
        } else {
            let nanos = i64::deserialize(deserializer)?;

            Ok(Duration::from_nanos(nanos))
        }
    }
}

impl<'de> Visitor<'de> for DurationVisitor {
    type Value = Duration;

    fn expecting(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(r#"a map { "nanos": i64 }"#)
    }

    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> Result<Duration, A::Error> {
        let mut nanos: Option<i64> = None;

        while let Some(key) = map.next_key::<DurationField>()? {
            match key {
                DurationField::Nanos => {
                    nanos = Some(map.next_value()?);
                }
            }
        }

        let nanos = nanos.ok_or_else(|| de::Error::missing_field("nanos"))?;

        Ok(Duration::from_nanos(nanos))
    }
}

impl<'de> Deserialize<'de> for DurationField {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DurationFieldVisitor;

        impl Visitor<'_> for DurationFieldVisitor {
            type Value = DurationField;

            fn expecting(
                &self,
                f: &mut fmt::Formatter<'_>,
            ) -> fmt::Result {
                f.write_str("`nanos`")
            }

            fn visit_str<E: de::Error>(
                self,
                v: &str,
            ) -> Result<DurationField, E> {
                match v {
                    "nanos" => Ok(DurationField::Nanos),
                    other => Err(de::Error::unknown_field(other, &["nanos"])),
                }
            }
        }

        deserializer.deserialize_identifier(DurationFieldVisitor)
    }
}

impl Serialize for DurationParts {
    fn serialize<Ser: Serializer>(
        &self,
        serializer: Ser,
    ) -> Result<Ser::Ok, Ser::Error> {
        if serializer.is_human_readable() {
            // { "seconds": 5, "nanos": 500000000 }
            let mut s = serializer.serialize_struct("DurationParts", 2)?;

            s.serialize_field("seconds", &self.seconds)?;
            s.serialize_field("nanos", &self.nanos)?;
            s.end()
        } else {
            // Compact: [u64, u32]
            let mut t = serializer.serialize_tuple(2)?;

            t.serialize_element(&self.seconds)?;
            t.serialize_element(&self.nanos)?;
            t.end()
        }
    }
}

impl<'de> Deserialize<'de> for DurationParts {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_struct(
                "DurationParts",
                DURATION_PARTS_FIELDS,
                DurationPartsMapVisitor,
            )
        } else {
            deserializer.deserialize_tuple(2, DurationPartsTupleVisitor)
        }
    }
}

impl<'de> Visitor<'de> for DurationPartsMapVisitor {
    type Value = DurationParts;

    fn expecting(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(r#"a map { "seconds": u64, "nanos": u32 }"#)
    }

    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> Result<DurationParts, A::Error> {
        let mut seconds: Option<u64> = None;
        let mut nanos: Option<u32> = None;

        while let Some(key) = map.next_key::<DurationPartsField>()? {
            match key {
                DurationPartsField::Seconds => {
                    seconds = Some(map.next_value()?);
                }
                DurationPartsField::Nanos => {
                    nanos = Some(map.next_value()?);
                }
            }
        }

        let seconds = seconds.ok_or_else(|| de::Error::missing_field("seconds"))?;
        let nanos = nanos.ok_or_else(|| de::Error::missing_field("nanos"))?;

        DurationParts::new(seconds, nanos).map_err(|e| match e {
            GnssTimeError::InvalidInput(msg) => de::Error::custom(msg),
            _ => de::Error::custom("invalid DurationParts"),
        })
    }
}

impl<'de> Deserialize<'de> for DurationPartsField {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DurationPartsFieldVisitor;

        impl Visitor<'_> for DurationPartsFieldVisitor {
            type Value = DurationPartsField;

            fn expecting(
                &self,
                f: &mut fmt::Formatter<'_>,
            ) -> fmt::Result {
                f.write_str("`seconds` or `nanos`")
            }

            fn visit_str<E: de::Error>(
                self,
                v: &str,
            ) -> Result<DurationPartsField, E> {
                match v {
                    "seconds" => Ok(DurationPartsField::Seconds),
                    "nanos" => Ok(DurationPartsField::Nanos),
                    other => Err(de::Error::unknown_field(other, DURATION_PARTS_FIELDS)),
                }
            }
        }

        deserializer.deserialize_identifier(DurationPartsFieldVisitor)
    }
}

impl<'de> Visitor<'de> for DurationPartsTupleVisitor {
    type Value = DurationParts;

    fn expecting(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str("a 2-element tuple [u64, u32]")
    }

    fn visit_seq<A: SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<DurationParts, A::Error> {
        let seconds: u64 = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let nanos: u32 = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;

        DurationParts::new(seconds, nanos).map_err(|e| match e {
            GnssTimeError::InvalidInput(msg) => de::Error::custom(msg),
            _ => de::Error::custom("invalid DurationParts"),
        })
    }
}

impl fmt::Display for ScaleMismatch<'_> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "scale mismatch: expected \"{}\", got \"{}\"",
            self.expected, self.got,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{std::string::ToString, Beidou, Galileo, Glonass, Gps, Tai, Utc};

    #[test]
    fn test_gps_serialize_json_exact_format() {
        let t = Time::<Gps>::from_seconds(1_356_566_418);
        let json = serde_json::to_string(&t).unwrap();

        assert_eq!(json, r#"{"scale":"GPS","nanos":1356566418000000000}"#);
    }

    #[test]
    fn test_gps_deserialize_json() {
        let json = r#"{"scale":"GPS","nanos":1356566418000000000}"#;
        let t: Time<Gps> = serde_json::from_str(json).unwrap();

        assert_eq!(t, Time::<Gps>::from_seconds(1_356_566_418));
    }

    #[test]
    fn test_gps_json_roundtrip_with_sub_second() {
        let original = Time::<Gps>::from_nanos(1_356_566_418_123_456_789);
        let json = serde_json::to_string(&original).unwrap();
        let back: Time<Gps> = serde_json::from_str(&json).unwrap();

        assert_eq!(original, back);
    }

    #[test]
    fn test_gps_epoch_json_roundtrip() {
        let t = Time::<Gps>::EPOCH;
        let json = serde_json::to_string(&t).unwrap();

        assert_eq!(json, r#"{"scale":"GPS","nanos":0}"#);

        let back: Time<Gps> = serde_json::from_str(&json).unwrap();

        assert_eq!(t, back);
    }

    #[test]
    fn test_gps_max_json_roundtrip() {
        let t = Time::<Gps>::MAX;
        let json = serde_json::to_string(&t).unwrap();
        let back: Time<Gps> = serde_json::from_str(&json).unwrap();

        assert_eq!(t, back);
    }

    #[test]
    fn test_utc_json_roundtrip() {
        let t = Time::<Utc>::from_nanos(1_514_764_800_000_000_000);
        let json = serde_json::to_string(&t).unwrap();
        let back: Time<Utc> = serde_json::from_str(&json).unwrap();

        assert_eq!(t, back);
    }

    #[test]
    fn test_tai_json_roundtrip() {
        let t = Time::<Tai>::from_seconds(100_000_000);
        let json = serde_json::to_string(&t).unwrap();
        let back: Time<Tai> = serde_json::from_str(&json).unwrap();

        assert_eq!(t, back);
    }

    #[test]
    fn test_galileo_json_roundtrip() {
        let t = Time::<Galileo>::from_nanos(999_999_999_999);
        let json = serde_json::to_string(&t).unwrap();
        let back: Time<Galileo> = serde_json::from_str(&json).unwrap();

        assert_eq!(t, back);
    }

    #[test]
    fn test_beidou_json_roundtrip() {
        let t = Time::<Beidou>::EPOCH;
        let json = serde_json::to_string(&t).unwrap();
        let back: Time<Beidou> = serde_json::from_str(&json).unwrap();

        assert_eq!(t, back);
    }

    #[test]
    fn test_glonass_json_roundtrip() {
        let t = Time::<Glonass>::from_nanos(42_000_000_000);
        let json = serde_json::to_string(&t).unwrap();
        let back: Time<Glonass> = serde_json::from_str(&json).unwrap();

        assert_eq!(t, back);
    }

    #[test]
    fn test_all_scales_json_scale_field() {
        let gps_v: serde_json::Value = serde_json::to_value(Time::<Gps>::EPOCH).unwrap();
        let utc_v: serde_json::Value = serde_json::to_value(Time::<Utc>::EPOCH).unwrap();
        let tai_v: serde_json::Value = serde_json::to_value(Time::<Tai>::EPOCH).unwrap();
        let gal_v: serde_json::Value = serde_json::to_value(Time::<Galileo>::EPOCH).unwrap();
        let bdt_v: serde_json::Value = serde_json::to_value(Time::<Beidou>::EPOCH).unwrap();
        let glo_v: serde_json::Value = serde_json::to_value(Time::<Glonass>::EPOCH).unwrap();

        assert_eq!(gps_v["scale"], "GPS");
        assert_eq!(utc_v["scale"], "UTC");
        assert_eq!(tai_v["scale"], "TAI");
        assert_eq!(gal_v["scale"], "GAL");
        assert_eq!(bdt_v["scale"], "BDT");
        assert_eq!(glo_v["scale"], "GLO");
    }

    #[test]
    fn test_scale_mismatch_gps_into_utc_fails() {
        let json = r#"{"scale":"GPS","nanos":0}"#;
        let result: Result<Time<Utc>, _> = serde_json::from_str(json);

        assert!(result.is_err(), "scale mismatch must fail");

        let msg = result.unwrap_err().to_string();

        assert!(
            msg.contains("GPS") || msg.contains("UTC") || msg.contains("scale"),
            "error should mention scale: {msg}"
        );
    }

    #[test]
    fn test_scale_mismatch_tai_into_gps_fails() {
        let json = r#"{"scale":"TAI","nanos":0}"#;
        let result: Result<Time<Gps>, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_without_scale_field_uses_nanos() {
        // scale field is optional — only validated when present
        let json = r#"{"nanos":12345678}"#;
        let t: Time<Gps> = serde_json::from_str(json).unwrap();

        assert_eq!(t.as_nanos(), 12_345_678);
    }

    #[test]
    fn test_deserialize_missing_nanos_field_fails() {
        let json = r#"{"scale":"GPS"}"#;
        let result: Result<Time<Gps>, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }

    #[test]
    fn test_gps_postcard_roundtrip() {
        let original = Time::<Gps>::from_nanos(1_356_566_418_123_456_789);
        let bytes = postcard::to_allocvec(&original).unwrap();
        let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(original, back);
    }

    #[test]
    fn test_utc_postcard_roundtrip() {
        let t = Time::<Utc>::from_nanos(1_514_764_800_000_000_000);
        let bytes = postcard::to_allocvec(&t).unwrap();
        let back: Time<Utc> = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(t, back);
    }

    #[test]
    fn test_gps_epoch_postcard_roundtrip() {
        let t = Time::<Gps>::EPOCH;
        let bytes = postcard::to_allocvec(&t).unwrap();
        let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(t, back);
    }

    #[test]
    fn test_gps_max_postcard_roundtrip() {
        let t = Time::<Gps>::MAX;
        let bytes = postcard::to_allocvec(&t).unwrap();
        let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(t, back);
    }

    #[test]
    fn test_postcard_is_smaller_than_json() {
        let t = Time::<Gps>::from_seconds(1_000_000);
        let json_len = serde_json::to_vec(&t).unwrap().len();
        let postcard_len = postcard::to_allocvec(&t).unwrap().len();

        assert!(
            postcard_len < json_len,
            "postcard ({postcard_len}B) should be smaller than JSON ({json_len}B)"
        );
    }

    #[test]
    fn test_duration_serialize_json_exact() {
        let d = Duration::from_seconds(-7);
        let json = serde_json::to_string(&d).unwrap();

        assert_eq!(json, r#"{"nanos":-7000000000}"#);
    }

    #[test]
    fn test_duration_zero_json() {
        let d = Duration::ZERO;
        let json = serde_json::to_string(&d).unwrap();

        assert_eq!(json, r#"{"nanos":0}"#);

        let back: Duration = serde_json::from_str(&json).unwrap();

        assert_eq!(d, back);
    }

    #[test]
    fn test_duration_json_roundtrip_cases() {
        let cases = [
            Duration::ZERO,
            Duration::from_seconds(42),
            Duration::from_seconds(-100),
            Duration::from_nanos(1),
            Duration::from_nanos(-1),
            Duration::from_millis(500),
            Duration::MAX,
            Duration::MIN,
        ];
        for d in cases {
            let json = serde_json::to_string(&d).unwrap();
            let back: Duration = serde_json::from_str(&json).unwrap();

            assert_eq!(d, back, "round-trip failed for {d:?}");
        }
    }

    #[test]
    fn test_duration_missing_nanos_field_fails() {
        let result: Result<Duration, _> = serde_json::from_str(r"{}");

        assert!(result.is_err());
    }

    #[test]
    fn test_duration_postcard_roundtrip() {
        let cases = [
            Duration::ZERO,
            Duration::from_seconds(1),
            Duration::from_seconds(-1),
            Duration::from_nanos(123_456_789),
            Duration::MAX,
            Duration::MIN,
        ];
        for d in cases {
            let bytes = postcard::to_allocvec(&d).unwrap();
            let back: Duration = postcard::from_bytes(&bytes).unwrap();

            assert_eq!(d, back);
        }
    }

    #[test]
    fn test_duration_parts_serialize_json_exact() {
        let p = DurationParts {
            seconds: 5,
            nanos: 500_000_000,
        };
        let json = serde_json::to_string(&p).unwrap();

        assert_eq!(json, r#"{"seconds":5,"nanos":500000000}"#);
    }

    #[test]
    fn test_duration_parts_zero_json() {
        let p = DurationParts {
            seconds: 0,
            nanos: 0,
        };
        let json = serde_json::to_string(&p).unwrap();

        assert_eq!(json, r#"{"seconds":0,"nanos":0}"#);

        let back: DurationParts = serde_json::from_str(&json).unwrap();

        assert_eq!(p, back);
    }

    #[test]
    fn test_duration_parts_json_roundtrip() {
        let cases = [
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
            DurationParts {
                seconds: 1,
                nanos: 0,
            },
            DurationParts {
                seconds: 86_400,
                nanos: 123_456_789,
            },
            DurationParts {
                seconds: 604_799,
                nanos: 999_999_999,
            },
        ];
        for p in cases {
            let json = serde_json::to_string(&p).unwrap();
            let back: DurationParts = serde_json::from_str(&json).unwrap();

            assert_eq!(p, back);
        }
    }

    #[test]
    fn test_duration_parts_invalid_nanos_rejected() {
        // nanos >= 1_000_000_000 must be rejected
        let json = r#"{"seconds":0,"nanos":1000000000}"#;
        let result: Result<DurationParts, _> = serde_json::from_str(json);

        assert!(result.is_err(), "nanos >= 1_000_000_000 must fail");
    }

    #[test]
    fn test_duration_parts_missing_seconds_fails() {
        let result: Result<DurationParts, _> = serde_json::from_str(r#"{"nanos":0}"#);

        assert!(result.is_err());
    }

    #[test]
    fn test_duration_parts_missing_nanos_fails() {
        let result: Result<DurationParts, _> = serde_json::from_str(r#"{"seconds":0}"#);

        assert!(result.is_err());
    }

    #[test]
    fn test_duration_parts_postcard_roundtrip() {
        let cases = [
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
            DurationParts {
                seconds: 86_400,
                nanos: 123_456_789,
            },
            DurationParts {
                seconds: u64::MAX / 1_000_000_000,
                nanos: 999_999_999,
            },
        ];

        for p in cases {
            let bytes = postcard::to_allocvec(&p).unwrap();
            let back: DurationParts = postcard::from_bytes(&bytes).unwrap();

            assert_eq!(p, back);
        }
    }

    #[test]
    fn test_from_week_tow_json_roundtrip() {
        let tow = DurationParts {
            seconds: 432_000,
            nanos: 0,
        };
        let gps = Time::<Gps>::from_week_tow(2345, tow).unwrap();

        // Both types roundtrip independently
        let gps_json = serde_json::to_string(&gps).unwrap();
        let tow_json = serde_json::to_string(&tow).unwrap();

        let gps_back: Time<Gps> = serde_json::from_str(&gps_json).unwrap();
        let tow_back: DurationParts = serde_json::from_str(&tow_json).unwrap();

        assert_eq!(gps, gps_back);
        assert_eq!(tow, tow_back);

        // Reconstructed GPS timestamp matches original
        let gps_from_tow = Time::<Gps>::from_week_tow(2345, tow_back).unwrap();

        assert_eq!(gps, gps_from_tow);
    }

    #[test]
    fn test_time_duration_combined_json() {
        // Simulate a "timestamp + offset" structure
        let t = Time::<Gps>::from_seconds(1_000_000);
        let d = Duration::from_seconds(3600);

        let t_json = serde_json::to_string(&t).unwrap();
        let d_json = serde_json::to_string(&d).unwrap();

        let t_back: Time<Gps> = serde_json::from_str(&t_json).unwrap();
        let d_back: Duration = serde_json::from_str(&d_json).unwrap();

        assert_eq!(t + d, t_back + d_back);
    }
}
