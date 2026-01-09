# Introduction

Welcome to the **blaise** documentation book.

**blaise** (/blɛz/) is an easy-to-use, fully local engine for public transit data with a strong focus on performance. It handles the heavy lifting of loading, searching, and routing through complex transit schedules so you can focus on building your application without relying on external, often expensive, APIs.

The project is named after **Blaise Pascal**, the French mathematician and physicist who created the first public transit system in Paris in 1662, known as the *Carrosses à cinq sols*.

## What is GTFS?

At its core, *blaise* is powered by **GTFS** (General Transit Feed Specification). GTFS is the global standard for representing public transit schedules and associated geographic information. A standard GTFS feed consists of a collection of CSV files (often compressed into a ZIP) containing:

- **Stops and Areas**: Physical locations where vehicles pick up or drop off passengers.
- **Routes**: Groups of trips that are displayed to riders under a single name (e.g., "Line 1").
- **Trips**: Specific journeys occurring at specific times.
- **Stop Times**: The exact arrival and departure times for every stop on every trip.
- **Transfers**: Rules for moving between different stops or routes.

*blaise* consumes these feeds and transforms them into a highly optimized, memory-efficient **Repository** designed for lightning-fast lookups.

## What is RAPTOR?

For pathfinding, *blaise* implements an optimized version of the **RAPTOR** (Round-Based Public Transit Routing) algorithm.

Unlike traditional graph-based algorithms, RAPTOR was designed specifically for transit networks. It operates in "rounds," where each round finds `K` all stops reachable using exactly `K` trips. This approach is naturally suited to the structure of transit schedules, where the "cost" of a leg isn't just distance, but time spent waiting for a specific vehicle.

## Why RAPTOR vs. Dijkstra/A*?

If you have ever used a standard pathfinding library, you are likely familiar with **Dijkstra's algorithm** or its heuristic-driven cousin, **A***. While these are excellent for road networks or video game pathfinding, they often struggle with public transit for several reasons:

### 1. The Time-Dependent Problem

In a road network, an edge between Point A and Point B usually has a static cost (the time it takes to drive). In public transit, the "cost" of an edge depends entirely on **when you arrive at the node**. If you arrive at a bus stop at 8:05 AM and the bus leaves at 8:10 AM, your cost is 5 minutes. If you arrive at 8:11 AM, your cost might be 30 minutes.

Dijkstra requires creating a massive "time-expanded" graph where every single bus departure is its own node, leading to astronomical memory usage and slow query times.

### 2. Multi-Criteria Optimization

Public transit riders rarely care *only* about the shortest time. They also care about the **number of transfers**.

- **Dijkstra** finds the single shortest path based on one weight.
- **RAPTOR** naturally finds a "Pareto-optimal" set of journeys. Because it moves in rounds, it can easily tell you: "Here is the fastest way to get there with 2 transfers, and here is a slightly slower way with 0 transfers."

### 3. Cache Locality and Performance

Standard graph algorithms involve "pointer chasing"—jumping around different parts of memory to follow edges.
RAPTOR, as implemented in *blaise*, uses flattened arrays and contiguous memory blocks. By scanning routes linearly, it takes full advantage of modern CPU caches and parallel execution via **Rayon**. This is how *blaise* can reduce routing times on large, complex datasets.
