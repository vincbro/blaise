# Getting Started

**blaise** is a Rust library: it embeds the routing engine directly into your binary, with no server and no network in between. See [Using the Library](./library.md).

## Prerequisites

You will need a **GTFS dataset** to get started. **blaise** does not come bundled with transit data, it acts as the engine to process the data you provide.

- **Standard GTFS**: You will typically need a `.zip` file containing the standard GTFS schedule files (e.g., `stops.txt`, `trips.txt`, `stop_times.txt`).
- **Local Storage**: Ensure you have enough disk space and memory to house the repository, as **blaise** builds a high-performance in-memory representation of the network.
