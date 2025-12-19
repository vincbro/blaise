![ontrack](./assets/ontrack_slim.png)

[![Crates.io](https://img.shields.io/crates/v/ontrack.svg)](https://crates.io/crates/ontrack)
[![Documentation](https://docs.rs/ontrack/badge.svg)](https://docs.rs/ontrack)
[![License](https://img.shields.io/crates/l/ontrack.svg)](LICENSE)

On Track is a high-performance Rust library designed to make transit data easy to work with.
It handles the heavy lifting of loading, searching, and routing through complex GTFS transit schedules so you can focus on building your application.

> [!NOTE]
> This project is early in development, if you like the idea and want to help improve it, please open an issue.

## Key Features

- **Simplified Integration**: Stop worrying about GTFS parsing; just point the library at a .zip file and start querying.
- **Reliable Routing**: Give your users accurate itineraries that account for every transfer and walking connection.
- **Search that Just Works**: Implement high-quality location search without needing external search engines.
- **Location Intelligence**: Easily connect coordinates to transit infrastructure to power "near me" features.
- **Efficient Resource Use**: Deploy on smaller, more cost-effective servers thanks to a highly optimized, low-memory design.

## Installation

Add On Track to your Cargo.toml:
```bash
cargo add ontrack
```

## Quick Start

```rust
use ontrack::gtfs::{Gtfs, Config};
use ontrack::repository::Repository;
use ontrack::router::{Router, graph::Location};
use ontrack::shared::time::Time;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gtfs = Gtfs::new(Config::default()).from_zip("transit_data.zip")?;
    let repo = Repository::new().with_gtfs(gtfs)?;

    let from = Location::Stop("STOP_ID_1".into());
    let to = Location::Coordinate(ontrack::shared::geo::Coordinate { latitude: 59.3, longitude: 18.0 });
    let departure = Time::from_seconds(36000); // 10:00 AM

    let itinerary = Router::new(repo, from, to, departure)?.run()?;

    println!("Found a route with {} legs!", itinerary.legs.len());
    Ok(())
} 
```

## Core Concepts

- **Repository**: Your central hub for transit data. It holds all the stops, routes, and schedules in a format optimized for speed.
- **Router**: The logic engine that finds the best path. It understands how to connect different bus or train lines with walking paths.
- **Shared Utilities**: Built-in tools for handling geographic distances and transit-specific time calculations.

## Roadmap

- [ ] Production-ready web server crate with docker image
- [ ] Multi-threaded routing for even faster results
- [ ] Real-time data updates (GTFS-RT)
- [ ] Advanced date and holiday filtering

## Refrences

- [GTFS Specification](https://gtfs.org/documentation/schedule/reference/)
- [Development Data (Sweden):](https://www.trafiklab.se/api/gtfs-datasets/gtfs-sweden/static-specification/)

## License

This project is licensed under the MIT License.
