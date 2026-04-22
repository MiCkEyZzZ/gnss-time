use gnss_time::prelude::*;

fn main() {
    let t = Time::<Gps>::MAX;

    let safe = t.saturating_add(Duration::from_seconds(1));

    // never panics on embedded systems
    assert_eq!(t, safe);

    println!("Safe arithmetic works without overflow");
}
