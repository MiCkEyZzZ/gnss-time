# gnss-time

**Type-safe time handling for GNSS systems in Rust.**

`gnss-time` is a minimal, zero-cost core library for working with time in satellite
navigation systems such as GLONASS, GPS, Galileo, and BeiDou.

This is **not** a GNSS engine and **not** an RTK library.
It is a strictly typed time model designed to prevent domain-mixing bugs at c
ompile time.

## Features

- **Type-safe time scales** — `Glonass`, `Gps`, `Galileo`, `Beidou`, `Tai`, `Utc`
- **Fixed epochs** for each system (1980-01-06, 1996-01-01, etc.)
- **Conversions via TAI** as a unified pivot
- **Domain-specific formats**
    - `Week:TOW` for GPS/Galileo/BeiDou
    - `Day:TOD` for GLONASS

- **High-precision durations** (`Duration`) with nanosecond resolution
- **Zero-cost abstractions** — timestamps are 8 bytes (`u64`)
- **`no_std` by default** — suitable for embedded systems
- **Explicit leap second handling** (no hidden global state)

## Example

```rust
use gnss_time::{Time, Duration, Gps};

let epoch = Time::<Gps>::EPOCH;
let one_week = Time::<Gps>::from_week_tow(1, 0.0).unwrap();
let diff = one_week - epoch;

assert_eq!(diff.as_seconds(), 604_800);
```

## Design Goals

- Prevent mixing incompatible time domains at compile time
- Make leap seconds explicit and impossible to ignore
- Provide zero-cost abstractions over raw timestamps
- Be fully usable in `no_std` environments
- Serve as a foundational building block for GNSS software

## Supported Time Scales

| Scale   | Epoch              | Format        | Offset vs TAI         |
| ------- | ------------------ | ------------- | --------------------- |
| GLONASS | 1996-01-01 UTC(SU) | `GLO D:TOD`   | contextual (needs LS) |
| GPS     | 1980-01-06 UTC     | `GPS W:TOW`   | +19 s (fixed)         |
| Galileo | 1999-08-22 UTC     | `GAL W:TOW`   | +19 s (fixed)         |
| BeiDou  | 2006-01-01 UTC     | `BDT W:TOW`   | +33 s (fixed)         |
| TAI     | 1958-01-01         | `TAI +Ss Nns` | 0 s (pivot)           |
| UTC     | 1972-01-01         | `UTC +Ss Nns` | contextual (needs LS) |

## Why not use standard libraries?

Typical time libraries:

- do not distinguish GNSS time domains
- allow unsafe mixing of GPS / UTC / TAI
- hide leap seconds or ignore them entirely
- are not `no_std` compatible

`gnss-time` solves these problems at the type level.

## Status

- [x] Core types (`Time<S>`, `Duration`)
- [x] Epoch definitions
- [x] Fixed-offset conversions
- [x] Leap second handling
- [x] Display formats
- [ ] Full conversion matrix
- [ ] Serialization (`serde`)
- [ ] Parsing (RINEX / NMEA / UBX)

## License

Dual-licensed under:

- MIT
- Apache-2.0

## Contributing

Contributions are welcome.
Focus areas:

- conversion correctness
- embedded support
- performance benchmarks
- real-world GNSS datasets
