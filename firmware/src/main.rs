#![no_std]
#![no_main]

use core::hint::black_box;

use cortex_m_rt::entry;
use gnss_time::{
    Duration, DurationParts, Gps, IntoScale, IntoScaleWith, LeapSeconds, Tai, Time, Utc,
};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// =============================================================================
// Probe functions: one symbol per operation so each can be measured
// independently. `black_box` prevents constant folding / dead-code elimination.
// =============================================================================

#[inline(never)]
fn probe_from_week_tow(
    week: u16,
    tow: DurationParts,
) -> Time<Gps> {
    match Time::<Gps>::from_week_tow(week, tow) {
        Ok(value) => value,
        Err(_) => loop {},
    }
}

#[inline(never)]
fn probe_into_scale(t: Time<Gps>) -> Time<Tai> {
    match t.into_scale() {
        Ok(value) => value,
        Err(_) => loop {},
    }
}

#[inline(never)]
fn probe_gps_to_utc(t: Time<Gps>) -> Time<Utc> {
    match t.into_scale_with(LeapSeconds::builtin()) {
        Ok(value) => value,
        Err(_) => loop {},
    }
}

#[inline(never)]
fn probe_time_saturating_add(
    t: Time<Gps>,
    d: Duration,
) -> Time<Gps> {
    t.saturating_add(d)
}

#[inline(never)]
fn probe_time_checked_add(
    t: Time<Gps>,
    d: Duration,
) -> Option<Time<Gps>> {
    t.checked_add(d)
}

#[entry]
fn main() -> ! {
    let gps = probe_from_week_tow(
        black_box(2345),
        black_box(DurationParts {
            seconds: 0,
            nanos: 0,
        }),
    );
    let _ = black_box(probe_into_scale(gps));
    let _ = black_box(probe_gps_to_utc(gps));
    let _ = black_box(probe_time_saturating_add(
        gps,
        black_box(Duration::from_seconds(1)),
    ));
    let _ = black_box(probe_time_checked_add(
        gps,
        black_box(Duration::from_seconds(1)),
    ));

    loop {}
}
