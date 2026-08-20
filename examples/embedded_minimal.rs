#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

use gnss_time::{DurationParts, Gps, IntoScale, IntoScaleWith, LeapSeconds, Tai, Time, Utc};

// =============================================================================
// Panic handler: only required on embedded targets (std provides one on host).
// =============================================================================

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// =============================================================================
// Probe functions: one symbol per operation, for `.text` size measurement.
// `black_box` prevents constant folding / dead-code elimination.
// =============================================================================

#[inline(never)]
fn probe_from_week_tow(
    week: u16,
    tow: DurationParts,
) -> Time<Gps> {
    Time::<Gps>::from_week_tow(week, tow).unwrap()
}

#[inline(never)]
fn probe_into_scale(t: Time<Gps>) -> Time<Tai> {
    t.into_scale().unwrap()
}

#[inline(never)]
fn probe_gps_to_utc(t: Time<Gps>) -> Time<Utc> {
    t.into_scale_with(LeapSeconds::builtin()).unwrap()
}

// =============================================================================
// Entry points.
// =============================================================================

#[cfg(target_arch = "arm")]
#[no_mangle]
pub extern "C" fn main() {
    let gps = probe_from_week_tow(
        core::hint::black_box(2345),
        core::hint::black_box(DurationParts {
            seconds: 0,
            nanos: 0,
        }),
    );
    let _ = core::hint::black_box(probe_into_scale(gps));
    let _ = core::hint::black_box(probe_gps_to_utc(gps));
}

#[cfg(not(target_arch = "arm"))]
fn main() {
    let gps = probe_from_week_tow(
        core::hint::black_box(2345),
        core::hint::black_box(DurationParts {
            seconds: 0,
            nanos: 0,
        }),
    );

    println!("GPS : {gps}");
    println!("TAI : {}", probe_into_scale(gps));
    println!("UTC : {}", probe_gps_to_utc(gps));
}
