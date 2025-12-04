use std::time::Instant;

use ontrack::{engine, gtfs};

#[test]
fn search_test() {
    let zip_path = format!("{}/tests/gtfs3.zip", env!("CARGO_MANIFEST_DIR"));
    let data = gtfs::Gtfs::new(gtfs::Config::default()).from_zip(zip_path.into());
    let engine = engine::Engine::new().with_gtfs(data).unwrap();

    let start = Instant::now();
    let area = engine.get_area("740098000").unwrap();
    let duration = start.elapsed();
    println!("get_area took: {:?}", duration);
    println!("Area name: {}", area.name);

    let start = Instant::now();
    let stops = engine.get_stops_in_area("740000001").unwrap();
    let duration = start.elapsed();
    println!("get_stops_in_area took: {:?}", duration);
    for stop in stops.iter() {
        println!(
            "Stop {} [{:?}] is in {}",
            stop.name, stop.location_type, area.name
        );
    }

    let start = Instant::now();
    let stop = engine.get_stop("9022050000001015").unwrap();
    let duration = start.elapsed();
    println!("get_stop took: {:?}", duration);
    println!(
        "Stop {} [{:?}] is in {}",
        stop.name, stop.location_type, area.name
    );
}
