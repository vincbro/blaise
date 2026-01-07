![blaise](./assets/blaise.png)


[![Crates.io](https://img.shields.io/crates/v/blaise.svg)](https://crates.io/crates/blaise)
[![Documentation](https://docs.rs/blaise/badge.svg)](https://docs.rs/blaise)
[![License](https://img.shields.io/crates/l/blaise.svg)](LICENSE)


*blaise* (/blɛz/) named after Blaise Pascal,
a French mathematician and physicist who created the
first public transit system in Paris in 1662 called the Carrosses à cinq sols.


*blaise* is an easy to use, fully local engine for public transit data with a strong focus on performance.
It handles the heavy lifting of loading, searching, and routing through GTFS transit schedules so you can focus on building your application without relying on external APIs.


Designed to be a complete local solution, *blaise* supports:

- **Fast routing**: Efficient schedule based pathfinding using a optimized version of the RAPTOR algorithm.
- **Fuzzy search**: Easily find stops and areas even with partial or imperfect names.
- **Geospatial search**: Discover transit options based on geographic coordinates.


> [!NOTE]
> This project is early in development, if you like the idea and want to help improve it, please open an issue.


## Server
While *blaise* is a library, we provide a server for projects that cannot directly integrate with the Rust crate.
The *blaise* server wraps this functionality in a ready to use REST API, supporting search, proximity queries, and routing out of the box.


[Read more](./crates/server/README.md)

## Installation

Add *blaise* to your Cargo.toml:
```bash
cargo add blaise
```

## Quick Start

```rust
use blaise::gtfs::{Gtfs, Config};
use blaise::repository::Repository;
use blaise::router::{Raptor, graph::Location};
use blaise::shared::{time::Time, geo::Coordinate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = Gtfs::new().from_zip("transit_data.zip")?;
    let repo = Repository::new().load_gtfs(data)?;

    let from = Location::Stop("STOP_ID_1".into());
    let to = Location::Coordinate(Coordinate { latitude: 59.3, longitude: 18.0 });
    let departure = Time::from_hms("16:00:00")?;

    let raptor =
        Raptor::new(repo, from, to).departure_at(departure);
    let itinerary = raptor.solve()?;
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
