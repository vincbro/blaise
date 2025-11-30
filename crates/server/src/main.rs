use std::{env, path::Path, process::exit, time::Instant};

use ontrack::{
    engine::{self, Identifiable},
    gtfs,
};

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 3 {
        println!("Missing gtfs zip");
        exit(1);
    }

    let path = Path::new(&args[1]).canonicalize().unwrap();

    let data = gtfs::Gtfs::new(gtfs::Config::default())
        .load_from_zip(path)
        .unwrap();
    let engine = engine::Engine::new().with_gtfs(data);

    let results = engine.search_areas_by_name(&args[2]);
    for value in results.iter().take(5) {
        println!("{}", value.name());
    }

    const RUNS: u32 = 1000;
    let start = Instant::now();
    for _ in 0..RUNS {
        // let area = engine.get_area(&args[2]).unwrap();
        // let stops = engine.get_stops_in_area(&args[2]).unwrap();
        let _ = engine.search_stops_by_name(&args[2]);
        // for value in result.iter().take(5) {
        //     println!("{}", value.name());
        // }
    }
    let duration = start.elapsed();
    println!("Operation took: {:?}", duration / RUNS);
}
