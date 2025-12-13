![ontrack](./assets/ontrack_slim.png)

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

## Implemented
- Load GTFS data directly from `.zip` archives.
- In-memory GTFS engine for fast read/query operations.
- Direct querying of entities by ID.
- Fuzzy search for stops and geographic areas.
- Distance-based search for stops and areas.
- Simple distance-based routing.
- Time-based routing and schedule-aware journey planning.

## Roadmap
- Server (`crates/server`)
- Multi threaded routing

## Refrences
- [GTFS Specification](https://gtfs.org/documentation/schedule/reference/)
- [Development Data (Sweden):](https://www.trafiklab.se/api/gtfs-datasets/gtfs-sweden/static-specification/)
