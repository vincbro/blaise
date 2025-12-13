use std::{
    env,
    fmt::format,
    io::{self, Write},
    path::Path,
    process::exit,
    time::Instant,
};

use ontrack::engine::{
    geo::Coordinate,
    routing::graph::{SearchState, SearchStateRef, Transition},
};
use ontrack::{
    engine::{self},
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

    let from = input("from");
    let from_area = engine.search_areas_by_name(&from)[0];
    let mut from_coordinate = Coordinate {
        latitude: 0.0,
        longitude: 0.0,
    };
    let from_stops = engine.stops_by_area_id(&from_area.id).unwrap();
    from_stops.iter().for_each(|stop| {
        from_coordinate.latitude += stop.coordinate.latitude;
        from_coordinate.longitude += stop.coordinate.longitude;
    });
    from_coordinate.latitude /= from_stops.len() as f64;
    from_coordinate.longitude /= from_stops.len() as f64;

    let to = input("to");
    let to_area = engine.search_areas_by_name(&to)[0];
    let mut to_coordinate = Coordinate {
        latitude: 0.0,
        longitude: 0.0,
    };
    let to_stops = engine.stops_by_area_id(&to_area.id).unwrap();
    to_stops.iter().for_each(|stop| {
        to_coordinate.latitude += stop.coordinate.latitude;
        to_coordinate.longitude += stop.coordinate.longitude;
    });
    to_coordinate.latitude /= to_stops.len() as f64;
    to_coordinate.longitude /= to_stops.len() as f64;
    let mut router = engine
        .router(from_coordinate.into(), to_coordinate.into())
        .unwrap();
    println!("Routing from: {} to: {}", from_area.name, to_area.name);

    let start = Instant::now();
    let route = router.run().unwrap();
    let duration = start.elapsed();
    println!("Routing took: {:?}", duration);

    let steps = route.len();

    let start = route.first().unwrap().clone();
    let end = route.last().unwrap().clone();

    println!("Started from: {}", get_name(&start, &engine));
    for state in route.into_iter().take(steps - 1).skip(1) {
        println!(
            "{} from {} to {}",
            get_mode(&state),
            get_name(&state.parent.clone().unwrap(), &engine),
            get_name(&state, &engine),
        )
    }
    println!("Ended at: {}", get_name(&end, &engine));
}

fn input(text: &str) -> String {
    let mut buf = String::new();
    print!("{text}: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut buf).unwrap();
    let search_str = buf.trim();
    search_str.to_string()
}

fn get_name(state: &SearchState, engine: &engine::Engine) -> String {
    match state.stop_idx {
        Some(stop_idx) => engine.stops[stop_idx].name.to_string(),
        None => format!(
            "{}, {}",
            state.coordinate.latitude, state.coordinate.longitude,
        ),
    }
}

fn get_mode(state: &SearchState) -> String {
    match state.transition {
        Transition::Travel { .. } => "Traveled".to_string(),
        Transition::Walk => "Walked".to_string(),
        Transition::Transfer { .. } => "Transfered".to_string(),
        Transition::Genesis => "Genesis".to_string(),
    }
}
