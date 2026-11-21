# 🚉 blaise

[![Crates.io](https://img.shields.io/crates/v/blaise.svg)](https://crates.io/crates/blaise)
[![Documentation](https://docs.rs/blaise/badge.svg)](https://docs.rs/blaise)
[![License](https://img.shields.io/crates/l/blaise.svg)](LICENSE-MIT)

A transit router that runs on your machine.

*blaise* (/blɛz/) is named after **Blaise Pascal**, the French mathematician and physicist who started the first public transit system in Paris in 1662, the Carrosses à cinq sols.

- No API keys, no rate limits, no network. Your queries never leave your process.
- One compiled file per network, memory-mapped. Nothing to parse at startup.
- Real-time delays, cancellations and skipped stops, when you have a GTFS-realtime feed.

## How it works

blaise is a library that plans journeys with [RAPTOR](https://www.microsoft.com/en-us/research/wp-content/uploads/2012/01/raptor_alenex.pdf), over a network compiled by [flatnet](https://github.com/vincbro/flatnet). flatnet turns a GTFS feed into one flat file once, and blaise reads it in place.

- **Fast routing.** Rounds run on a rayon pool, and a query allocates nothing that outlives it except the itinerary.
- **Both directions.** Ask for the best trip leaving at a time, or arriving by one.
- **Walking and transfers.** A coordinate can be an origin or a destination. A spatial grid finds the stops around it.
- **Pareto-optimal.** A route is judged on more than the clock, so a slightly later trip with fewer changes can win.
- **Service days.** Weekly calendars and holiday exceptions pick the trips. A trip of the day before that runs past midnight, and tomorrow's first trips, are both in reach. `Itinerary::date` is the date its times count from, and a time past 24:00:00 is on the day after, as in GTFS.
- **Live.** Delays move the times, canceled trips and skipped stops are not used, and every time in the itinerary carries both the scheduled and the expected value.

Fuzzy stop search lives in flatnet, next to the data it searches.

## Installation

```bash
cargo add blaise-core
```

## Quick start

Compile your feed once:

```bash
flatnet -i gtfs.zip -o network.flnt
```

Then map the file and ask for a route:

```rust
use std::fs::File;

use blaise_core::{
    raptor::{
        query::{QueryLocation, RaptorQuery},
        state::State,
    },
    spatial::SpatialGrid,
};
use flatnet_core::{
    consumer::Consumer,
    models::{Coordinate, Time},
};
use memmap2::Mmap;

fn main() -> anyhow::Result<()> {
    let file = File::open("network.flnt")?;
    // SAFETY: the file is only read, and nothing modifies it while it is mapped.
    let mmap = unsafe { Mmap::map(&file)? };
    let consumer = Consumer::new(&mmap)?;
    let spatial = SpatialGrid::new(&consumer);
    let mut state = State::new(&consumer);

    let from = QueryLocation::Coordinate(Coordinate::new(59.58, 17.89));
    let to = QueryLocation::Coordinate(Coordinate::new(59.34, 18.04));
    let departure = Time::from_hms("16:00:00").expect("a valid time");

    let itinerary = RaptorQuery::new(from, to)
        .with_departure(departure)
        .solve(&consumer, &spatial, &mut state)?;
    println!("Found a path with {} legs!", itinerary.legs.len());
    Ok(())
}
```

`State` and `SpatialGrid` are built once and reused for every query.

### Realtime

Apply GTFS-realtime feeds with flatnet and hand the result to the query:

```rust
use flatnet_core::rt::{self, Realtime, RealtimeBuilder};

let mut realtime = Realtime::for_service_date(&consumer, query.date);
let message = rt::decode_feed(&std::fs::read("TripUpdates.pb")?)?;
RealtimeBuilder::new(&consumer).apply([message].into_iter(), &mut realtime);

let itinerary = query.with_realtime(&realtime).solve(&consumer, &spatial, &mut state)?;
```

## Command line

The `blaise` binary plans one route and prints the itinerary, which is handy for trying a network out. Every file after the network is a GTFS-realtime feed, applied in order:

```bash
cargo run --release -p blaise -- network.flnt [TripUpdates.pb ...]
```

## Development

You need [rustup](https://rustup.rs) and [just](https://github.com/casey/just). The coding rules are in the `code-standards/` submodule, so clone with `--recurse-submodules`.

```sh
just toolchain   # the pinned Rust, nightly rustfmt, the cargo tools and the git hooks
just check       # what CI runs: fmt, clippy, docs, tests, dependencies
just check-full  # the same, with the property tests run for 40000 cases
just bench       # a route query timed forward and backward, on bench.flnt
```

Run `just` on its own to list every recipe. [STANDARDS.local.md](STANDARDS.local.md) holds what is specific to this project: the hot paths and the measured performance problems.

## Roadmap

- [x] Multi-threaded routing (RAPTOR)
- [x] Arrival and departure time constraints
- [x] Real-time data updates (GTFS-RT)
- [x] Service dates: `calendar.txt`, `calendar_dates.txt` exceptions and holidays
- [x] Trips that run past midnight, and journeys that cross it

## References

- [GTFS Specification](https://gtfs.org/documentation/schedule/reference/)
- [Development data (Sweden)](https://www.trafiklab.se/api/gtfs-datasets/gtfs-sweden/static-specification/)
- [RAPTOR](https://www.microsoft.com/en-us/research/wp-content/uploads/2012/01/raptor_alenex.pdf)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
