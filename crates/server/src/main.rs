use std::{
    env,
    io::{self, Write},
    path::Path,
    process::exit,
    time::Instant,
};

use ontrack::{
    engine::{self, Identifiable},
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

    dbg!(engine.stop_by_id("9022050010353002").unwrap());

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
        for value in results.iter().take(5) {
            println!("{}", value.name());
        }
        println!("Search took: {:?}", duration);
        buf.clear();
    }
}
