# Using the Library

Integrating **blaise** directly as a library allows you to build high-performance transit features into your Rust applications with minimal overhead. By embedding the engine, you avoid network latency and gain full control over the routing lifecycle.

## Installation

Add **blaise** to your project using Cargo:

```bash
cargo add blaise

```

## Core Workflow

Using the library typically involves four steps: loading raw GTFS data, building a searchable repository, configuring a router, and solving for an itinerary.

### 1. Loading GTFS Data

First, you must parse your transit data. **blaise** supports loading directly from a standard GTFS `.zip` file:

```rust
use blaise::gtfs::Gtfs;

let data = Gtfs::new().from_zip("transit_data.zip")?;

```

### 2. Building the Repository

The `Repository` is the optimized, in-memory data store used for routing. You hydrate it using the loaded GTFS data:

```rust
use blaise::repository::Repository;

let repo = Repository::new().load_gtfs(data)?;

```

### 3. Initializing the Router

To find a path, you create a `Raptor` instance. This requires a reference to your repository, a starting location, and a destination:

```rust
use blaise::raptor::{Raptor, Location};
use blaise::shared::geo::Coordinate;
use blaise::shared::Time;

let from = Location::Stop("STOP_ID_1".into());
let to = Location::Coordinate(Coordinate { latitude: 59.3, longitude: 18.0 });

let raptor = Raptor::new(&repo, from, to)
    .departure_at(Time::from_hms("16:00:00")?);
// OR
let raptor = repo::router(from, to)
    .departure_at(Time::from_hms("16:00:00")?);
```

### 4. Solving the Path

Finally, call `solve()` to execute the RAPTOR algorithm and retrieve your itinerary:

```rust
let itinerary = raptor.solve()?;
println!("Found a path with {} legs!", itinerary.legs.len());

```

## Full Example

Below is a complete implementation showing the library in action:

```rust
use blaise::gtfs::Gtfs;
use blaise::repository::Repository;
use blaise::raptor::{Raptor, Location};
use blaise::shared::{time::Time, geo::Coordinate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load data
    let data = Gtfs::new().from_zip("transit_data.zip")?;
    
    // 2. Build optimized repository
    let repo = Repository::new().load_gtfs(data)?;

    // 3. Configure search parameters
    let from = Location::Stop("STOP_ID_1".into());
    let to = Location::Coordinate(Coordinate { latitude: 59.3, longitude: 18.0 });
    let departure = Time::from_hms("16:00:00")?;

    // 4. Solve using RAPTOR
    let raptor = Raptor::new(&repo, from, to).departure_at(departure);
    let itinerary = raptor.solve()?;

    Ok(())
}

```

## Key Components

* **`Repository`**: A read-only, flattened data structure optimized for CPU cache locality and parallel access.
* **`Location`**: A flexible enum allowing you to route between specific Stops, Areas, or raw GPS Coordinates.
* **`Time`**: A specialized type representing seconds since midnight, used for all schedule calculations.
* **`Itinerary`**: The result of a successful search, containing a series of "Legs" (Walk or Transit) that describe the journey.
