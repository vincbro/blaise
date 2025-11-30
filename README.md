# On Track

On Track is a high-performance Rust library for loading, routing, searching, and querying GTFS transit data,
designed for minimal runtime allocations and fast lookups.

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
