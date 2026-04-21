# Примеры GNSS-времени

Этот каталог содержит примеры, демонстрирующие, как использовать crate `gnss-time`.

## Примеры

| Пример                        | Описание                                                             |
| ----------------------------- | -------------------------------------------------------------------- |
| `basic_usage.rs`              | Создание моментов времени, арифметика, разности, saturating-операции |
| `gps_week_tow.rs`             | Конвертации номера недели GPS / времени недели (TOW)                 |
| `glonass_day_tod.rs`          | Конвертации номера дня GLONASS / времени суток (TOD)                 |
| `glonass_receiver.rs`         | Обработка времени GNSS-приёмника GLONASS в реальном формате          |
| `receiver_timestamp.rs`       | Разбор timestamp от GNSS receiver (week + TOW + sub-ns)              |
| `multi_constellation.rs`      | Работа с несколькими GNSS системами (GPS / GAL / BDT / GLONASS)      |
| `scale_conversion.rs`         | Конвертация между шкалами времени через TAI pivot                    |
| `sync_alignment.rs`           | Проверка синхронизации и выравнивания времени между системами        |
| `log_stream.rs`               | Потоковая обработка GNSS временных логов                             |
| `embedded_safe_arithmetic.rs` | Безопасная арифметика без переполнений для embedded                  |
| `display_formats.rs`          | Различные форматы отображения для каждой шкалы времени               |
| `no_domain_mixing.rs`         | Демонстрация запрета смешивания разных time domain на уровне типов   |

## Запуск примеров

```bash
cargo run --example basic_usage
cargo run --example gps_week_tow
cargo run --example glonass_day_tod
cargo run --example glonass_receiver
cargo run --example receiver_timestamp
cargo run --example multi_constellation
cargo run --example scale_conversion
cargo run --example sync_alignment
cargo run --example log_stream
cargo run --example embedded_safe_arithmetic
cargo run --example display_formats
cargo run --example no_domain_mixing
```
