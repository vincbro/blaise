# Using the Library

Integrating **blaise** directly as a library allows you to build high-performance transit features into your Rust applications with minimal overhead. By embedding the engine, you avoid network latency and gain full control over the routing lifecycle.

## Installation

```bash
cargo add blaise gtfs-bin memmap2
```

## Core Workflow

Using the library involves four steps: compiling the feed once, mapping the compiled file, describing a journey, and solving for an itinerary.

### 1. Compiling GTFS Data

Blaise reads networks compiled by [gtfs-bin](https://github.com/vincbro/gtfs-bin). Compile a feed, a directory or a `.zip`, once:

```bash
gtfs-bin -i transit_data.zip -o network.gtfs
```

### 2. Mapping the Network

The `Consumer` reads the compiled file straight from a memory map, so there is nothing to load. `SpatialGrid` finds stops near a coordinate, and `State` is the working memory of a search. Build both once and reuse them:

```rust
use std::fs::File;

use blaise::{raptor::state::State, spatial::SpatialGrid};
use gtfs_bin::consumer::Consumer;
use memmap2::Mmap;

let file = File::open("network.gtfs")?;
// SAFETY: the file is only read, and nothing modifies it while it is mapped.
let mmap = unsafe { Mmap::map(&file)? };
let consumer = Consumer::new(&mmap)?;
let spatial = SpatialGrid::new(&consumer);
let mut state = State::new(&consumer);
```

### 3. Describing the Journey

A `RaptorQuery` takes an origin and a destination, each a stop, a list of stops or a coordinate:

```rust
use blaise::raptor::query::{QueryLocation, RaptorQuery};
use gtfs_bin::models::{Coordinate, Time};

let from = QueryLocation::Coordinate(Coordinate::new(59.58, 17.89));
let to = QueryLocation::Coordinate(Coordinate::new(59.34, 18.04));

let query = RaptorQuery::new(from, to).with_departure(Time::from_hms("16:00:00").expect("a valid time"));
// Or plan back from the time you must arrive by:
// let query = RaptorQuery::new(from, to).with_arrival(Time::from_hms("18:00:00").expect("a valid time"));
```

The query leaves now, today, unless you set `with_departure`, `with_arrival` or `with_date`.

### 4. Solving the Path

```rust
let itinerary = query.solve(&consumer, &spatial, &mut state)?;
println!("Found a path with {} legs!", itinerary.legs.len());
```

## Realtime

Build a `Realtime` for the service date with gtfs-bin, apply the feeds you have, and attach it to the query. Canceled trips and skipped stops are never used, and a delayed vehicle is boarded at the time it is expected. Each time in the itinerary is a `LiveTime` with the `scheduled` and `actual` values.

```rust
use gtfs_bin::rt::{self, Realtime, RealtimeBuilder};

let mut realtime = Realtime::for_service_date(&consumer, query.date);
let message = rt::decode_feed(&std::fs::read("TripUpdates.pb")?)?;
RealtimeBuilder::new(&consumer).apply([message].into_iter(), &mut realtime);

let itinerary = query
    .with_realtime(&realtime)
    .solve(&consumer, &spatial, &mut state)?;
```

A `Realtime` made for another service date than the query's is ignored.

## Key Components

* **`Consumer`**: A read-only, memory-mapped view of the compiled network.
* **`SpatialGrid`**: Finds the stops near a coordinate.
* **`State`**: The working memory of a search, reused between queries.
* **`QueryLocation`**: A stop, a list of stops or a raw GPS coordinate.
* **`Itinerary`**: The result of a search, a series of legs (walk, transfer or transit) that describe the journey.
