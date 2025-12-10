use std::{
    env,
    io::{self, Write},
    path::Path,
    process::exit,
    time::Instant,
};

use ontrack::{
    engine::{
        self, Identifiable,
        geo::{Coordinate, Distance},
    },
    gtfs,
};

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        println!("Missing gtfs zip");
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

    let to = Coordinate {
        latitude: 59.58364219722381,
        longitude: 17.893745024986465,
    };

    let from = engine.stop_by_id("43915").unwrap();

    let router = engine
        .router()
        .with_start_stop(from)
        .with_end_coordinate(to);

    router.run();

    let mut buf = String::new();
    loop {
        print!("Search: ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut buf).unwrap();
        let search_str = buf.trim();
        println!("Seaching for {search_str}...");
        let start = Instant::now();
        let results = engine.search_areas_by_name(search_str);
        let duration = start.elapsed();
        results.iter().take(5).for_each(|area| {
            println!("{}", area.name());
            let stops = engine.stops_by_area_id(&area.id).unwrap_or_default();
            // dbg!(&stops);
            stops.iter().for_each(|stop| {
                println!("  {}", stop.id);
                let trips = engine.trips_by_stop_id(&stop.id).unwrap_or_default();
                // dbg!(&trips);
                trips.iter().for_each(|trip| {
                    if let Some(headsign) = &trip.headsign {
                        println!("  {headsign}");
                    }
                });
            });
        });
        println!("Search took: {:?}", duration);
        buf.clear();
    }
}
