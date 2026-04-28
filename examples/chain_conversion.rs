use gnss_time::{matrix::beidou_via_gps_to_glonass_via_utc, Beidou, LeapSeconds, Time};

fn main() {
    let ls = LeapSeconds::builtin();
    let bdt = Time::<Beidou>::from_seconds(2_000_000_000); // достаточно большое значение

    match beidou_via_gps_to_glonass_via_utc(bdt, &ls) {
        Ok(chain) => {
            println!("Исходное BeiDou: {}", bdt);
            println!("-> GPS:          {}", chain.gps);
            println!("-> GLONASS:      {}", chain.glonass);
            println!("-> UTC:          {}", chain.utc);
            println!("-> TAI:          {}", chain.tai);
        }
        Err(e) => println!("Ошибка: {}", e),
    }
}
