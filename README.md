![blaise](./assets/blaise.png)


[![Crates.io](https://img.shields.io/crates/v/blaise.svg)](https://crates.io/crates/blaise)
[![Documentation](https://docs.rs/blaise/badge.svg)](https://docs.rs/blaise)
[![License](https://img.shields.io/crates/l/blaise.svg)](LICENSE)


*blaise* (/blɛz/) named after **Blaise Pascal**,
a French mathematician and physicist who created the
first public transit system in Paris in 1662 called the Carrosses à cinq sols.


*blaise* is a high-performance, fully local transit engine. It removes the "bottleneck" of developing transit applications and simulations by handling the complex infrastructure of schedule processing and routing entirely on your own hardware.


## Why blaise?

Developing apps that use public transit data usually means dealing with high-latency external APIs or the overwhelming complexity of raw GTFS files. blaise provides a better way:

- **Total Control**: No API keys, no rate limits, and no "black-box" routing. You own the code and the data.

- **Built for Scale**: Designed for simulations and high-traffic tools, allowing you to run thousands of queries per second with zero network latency.

- **Privacy First**: Your users' location data and search queries never leave your environment.

- **Transparent Logic**: Built in Rust for speed and predictability. The relationship between stops, trips, and routes is clear and easy to query.


## Core Features
- **Fast Routing**: Efficient schedule-based pathfinding using a highly optimized version of the RAPTOR algorithm.

- **Smart Search**: A built-in fuzzy search engine that handles partial names, typos, and abbreviations for stops and areas.

- **Geospatial Lookups**: Instant discovery of transit options based on geographic coordinates using spatial indexing.

> [!NOTE]
> This project is early in development, if you like the idea and want to help improve it, please open an issue.


## Server
While *blaise* is a library, there is also a standalone server for projects that cannot directly integrate with the Rust crate. The *blaise server* wraps this functionality in a ready-to-use REST API, supporting search, proximity queries, and routing out of the box.

[Read more](./crates/server/README.md)

## Installation

```bash
cargo add blaise
```

## Quick Start

```rust
use blaise::gtfs::{GtfsReader};
use blaise::repository::Repository;
use blaise::raptor::{Raptor, Location};
use blaise::shared::{time::Time, geo::Coordinate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = GtfsReader::new().from_zip("gtfs_data.zip")?;
    let repo = Repository::new().load_gtfs(reader)?;

    let from = Location::Stop("STOP_ID_1".into());
    let to = Location::Coordinate(Coordinate { latitude: 59.3, longitude: 18.0 });
    let departure = Time::from_hms("16:00:00")?;

    let raptor = Raptor::new(&repo, from, to).departure_at(departure);
    let itinerary = raptor.solve()?;
    println!("Found a path with {} legs!", itinerary.legs.len());
    Ok(())
}
```

## Roadmap

- [x] Web server crate with docker image
- [x] Multi-threaded routing (Switching to RAPTOR)
- [x] Arrival and departure time constraints for routing
- [ ] Real-time data updates (GTFS-RT)
- [ ] Advanced date and holiday filtering

## Refrences

- [GTFS Specification](https://gtfs.org/documentation/schedule/reference/)
- [Development Data (Sweden)](https://www.trafiklab.se/api/gtfs-datasets/gtfs-sweden/static-specification/)
- [RAPTOR](https://www.microsoft.com/en-us/research/wp-content/uploads/2012/01/raptor_alenex.pdf)

## License

This project is licensed under the MIT License.
