# Examples

All examples compile and run with `cargo run --example $name`.

## Getting started

| Example                                   | Description                                             |
| ----------------------------------------- | ------------------------------------------------------- |
| [`basic_usage`](basic_usage.rs)           | Create timestamps, add durations, compute deltas        |
| [`gps_week_tow`](gps_week_tow.rs)         | GPS week + TOW constructor and accessors                |
| [`glonass_day_tod`](glonass_day_tod.rs)   | GLONASS day + TOD constructor and accessors             |
| [`display_formats`](display_formats.rs)   | All `Display` formats: `Week:TOW`, `Day:TOD`, `+Ss Nns` |
| [`no_domain_mixing`](no_domain_mixing.rs) | Compile-time error: mixing GPS and GLONASS              |

## Conversions

| Example                                         | Description                                                |
| ----------------------------------------------- | ---------------------------------------------------------- |
| [`convert_basic`](convert_basic.rs)             | Fixed-offset conversions: GPS→TAI, GPS→Galileo, GPS→BeiDou |
| [`convert_contextual`](convert_contextual.rs)   | GPS↔UTC with leap second context, ambiguity detection      |
| [`scale_conversion`](scale_conversion.rs)       | Full conversion tour including overflow handling           |
| [`chain_conversion`](chain_conversion.rs)       | BeiDou→GPS→GLONASS→UTC→TAI in one call                     |
| [`multi_constellation`](multi_constellation.rs) | Same physical moment in GPS, Galileo, BeiDou               |
| [`sync_alignment`](sync_alignment.rs)           | Cross-scale alignment check                                |

## Receiver integration

| Example                                       | Description                       |
| --------------------------------------------- | --------------------------------- |
| [`receiver_timestamp`](receiver_timestamp.rs) | Parse u-blox GPS week+TOW output  |
| [`glonass_receiver`](glonass_receiver.rs)     | Parse GLONASS ephemeris day+TOD   |
| [`log_stream`](log_stream.rs)                 | Format a stream of GPS timestamps |

## Embedded / safe arithmetic

| Example                                                   | Description                     |
| --------------------------------------------------------- | ------------------------------- |
| [`embedded_safe_arithmetic`](embedded_safe_arithmetic.rs) | `saturating_add` — never panics |

## Advanced

| Example                                       | Description                                             |
| --------------------------------------------- | ------------------------------------------------------- |
| [`matrix_inspection`](matrix_inspection.rs)   | Runtime conversion graph: `ConversionMatrix`, `ScaleId` |
| [`dynamic_conversion`](dynamic_conversion.rs) | Dispatch conversion at runtime via `ScaleId`            |
