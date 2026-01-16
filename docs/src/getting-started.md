# Getting Started

To begin using **blaise**, you first need to decide how you want to integrate it into your workflow. Because **blaise** is designed as a complete local solution for GTFS data, we provide two primary ways to use it: as a high-performance Rust library or as a standalone web server.

## Choose Your Path

| Integration Style | Best For... | Link |
| --- | --- | --- |
| **Rust Library** | High-performance, low-latency applications where you want to embed the routing engine directly into your Rust binary. | [Using the Library](./library.md) |
| **REST Server** | Projects written in other languages (Node.js, Python, Go, etc.) or microservice architectures that need a ready-to-use API. | [Using the Server](./server.md) |


## Prerequisites

Regardless of which path you choose, you will need a **GTFS dataset** to get started. **blaise** does not come bundled with transit data, it acts as the engine to process the data you provide.

- **Standard GTFS**: You will typically need a `.zip` file containing the standard GTFS schedule files (e.g., `stops.txt`, `trips.txt`, `stop_times.txt`).
- **Local Storage**: Ensure you have enough disk space and memory to house the repository, as **blaise** builds a high-performance in-memory representation of the network.
