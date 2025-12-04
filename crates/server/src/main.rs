use std::{env, path::Path, process::exit, time::Instant};

use ontrack::{
    engine::{self, Identifiable},
    gtfs,
};

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 3 {
        println!("Missing gtfs zip and/or search string");
        exit(1);
    }

    let path = Path::new(&args[1]).canonicalize().unwrap();

    let start = Instant::now();
    let data = gtfs::Gtfs::new(gtfs::Config::default())
        .from_zip(path)
        .unwrap();
    let engine = engine::Engine::new().with_gtfs(data).unwrap();
    let duration = start.elapsed();
    println!("Loading took: {:?}", duration);

    dbg!(engine.get_stop("9022050010353002").unwrap());
    let start = Instant::now();
    let results = engine.search_areas_by_name(&args[2]);
    for value in results.iter().take(5) {
        println!("{}", value.name());
    }
    let duration = start.elapsed();
    println!("Operation took: {:?}", duration);
}
