# Примеры GNSS-времени

Этот каталог содержит примеры, демонстрирующие, как использовать crate `gnss-time`.

## Примеры

| Пример                | Описание                                                             |
| --------------------- | -------------------------------------------------------------------- |
| `basic_usage.rs`      | Создание моментов времени, арифметика, разности, saturating-операции |
| `gps_week_tow.rs`     | Конвертации номера недели GPS / времени недели (TOW)                 |
| `glonass_day_tod.rs`  | Конвертации номера дня GLONASS / времени суток (TOD)                 |
| `scale_conversion.rs` | Конвертация между шкалами времени (GPS ↔ Galileo ↔ BeiDou) через TAI |
| `display_formats.rs`  | Различные форматы отображения для каждой шкалы времени               |

## Запуск примеров

```bash
cargo run --example basic_usage
cargo run --example gps_week_tow
cargo run --example glonass_day_tod
cargo run --example scale_conversion
cargo run --example display_formats
```
