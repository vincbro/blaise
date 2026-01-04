![ontrack](./assets/ontrack.png)

[![Crates.io](https://img.shields.io/crates/v/ontrack.svg)](https://crates.io/crates/ontrack)
[![Documentation](https://docs.rs/ontrack/badge.svg)](https://docs.rs/ontrack)
[![License](https://img.shields.io/crates/l/ontrack.svg)](LICENSE)

On Track is a high-performance Rust library designed to make transit data easy to work with.
It handles the heavy lifting of loading, searching, and routing through GTFS transit schedules so you can focus on building your application.

> [!NOTE]
> This project is early in development, if you like the idea and want to help improve it, please open an issue.

## Server
While On Track is a Rust library, we provide a server for projects that cannot directly integrate with the Rust crate.

The On Track Server wraps the library's performance in a ready-to-use HTTP API, supporting search, proximity queries, and routing out of the box.

[Read more](./crates/server/README.md)

## Installation

Add On Track to your Cargo.toml:
```bash
cargo add ontrack
```

## Quick Start

```rust
use ontrack::gtfs::{Gtfs, Config};
use ontrack::repository::Repository;
use ontrack::router::{Raptor, graph::Location};
use ontrack::shared::{time::Time, geo::Coordinate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gtfs = Gtfs::new(Config::default()).from_zip("transit_data.zip")?;
    let repo = Repository::new().with_gtfs(gtfs)?;

    let from = Location::Stop("STOP_ID_1".into());
    let to = Location::Coordinate(Coordinate { latitude: 59.3, longitude: 18.0 });
    let departure = Time::from_hms("16:00:00");

    let itinerary = Raptor::new(repo, from, to, departure)?.solve()?;
    println!("Found a path with {} legs!", itinerary.legs.len());
    Ok(())
} 
```

## Roadmap

- [ ] Production-ready web server crate with docker image
- [x] Multi-threaded routing (Switching to RAPTOR)
- [ ] Real-time data updates (GTFS-RT)
- [ ] Advanced date and holiday filtering

## Refrences

- [GTFS Specification](https://gtfs.org/documentation/schedule/reference/)
- [Development Data (Sweden)](https://www.trafiklab.se/api/gtfs-datasets/gtfs-sweden/static-specification/)
- [RAPTOR](https://www.microsoft.com/en-us/research/wp-content/uploads/2012/01/raptor_alenex.pdf)

## License

This project is licensed under the MIT License.
