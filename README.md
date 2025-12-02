![ontrack](./assets/ontrack_github.png)

On Track is a high-performance Rust library for loading, routing, searching, and querying GTFS transit data,
designed for minimal runtime allocations and fast lookups.

> [!NOTE]
> This project is early in development, if you like the idea and want to help improve it, please open an issue.

## Installation
```bash
cargo add ontrack
```

## Design
The `ontrack` `engine` is designed to be **immutable** and **thread-safe**. All data is stored only once on the heap, and the engine works by passing around references rather than cloning or reallocating existing structures.


Entities such as `stops`, `areas`, and others are stored as `Arc<[T]>` slices, and all strings are held as `Arc<str>`. This ensures thread safety and keeps the memory footprint low, since no entity is ever allocated more than once.


The only time the `engine` allocates new memory is when performing request-driven operations such as **search** operations. In those cases, the newly allocated memory is owned entirely by the consumer (i.e., you). In a scenario like a web server, this means the allocated data exists only for the duration of the request and is freed immediately afterward.

## Usage
Simple program for finding areas (An area is a geografic area/collection of stops).
```rust
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

    // Takes the search string from a possible local space like data/gtfs.zip into system i.e /home/user/data/gtfs.zip
    let path = Path::new(&args[1]).canonicalize().unwrap();

    // The gtfs module is used to read and parse gtfs data into in memory structs.
    // The structs that are held in the gtfs are supposed to be identical to the ones found in the gtfs spec.
    // We don't load the data until a load_from function is called.
    let data = gtfs::Gtfs::new(gtfs::Config::default())
        .load_from_zip(path)
        .unwrap();


    // This creates a new engine, good to note is that the engine is empty until a with function is called.
    let engine = engine::Engine::new().with_gtfs(data);

    let start = Instant::now();
    // The return of the search_areas_by_name is owned by the main function.
    let results = engine.search_areas_by_name(&args[2]);
    // Since the area struct is built using Arc<str> it's really cheap to copy but here we are using refrences.
    for value in results.iter().take(5) {
        println!("{}", value.name());
    }
    let duration = start.elapsed();
    // Note that building with --release tag improves performance alot.
    println!("Operation took: {:?}", duration);
}
```

## Implemented
- Load GTFS data directly from `.zip` archives.
- In-memory GTFS engine for fast read/query operations.
- Direct querying of entities by ID. *500ns to 500µs*
- Fuzzy search for stops and geographic areas.

## Roadmap
- Distance-based search for stops and areas.
- Simple distance-based routing.
- Time-based routing and schedule-aware journey planning.

## Refrences
- [GTFS Specification](https://gtfs.org/documentation/schedule/reference/)
- [Development Data (Sweden):](https://www.trafiklab.se/api/gtfs-datasets/gtfs-sweden/static-specification/)
