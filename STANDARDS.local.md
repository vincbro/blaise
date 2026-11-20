# Project standards

What is specific to this project. The shared rules are in `code-standards/`. Where this file disagrees with them, this file wins.

## Commands

- **Gate** (must pass before any change is done): `just check`
- **Full gate** (the gate with every property test run for 40000 cases, for after touching `State`, `SpatialGrid` or `ParetoFront`): `just check-full`
- **Commit-message check** (takes a message file): `just commit-msg <file>`
- **Benchmarks** (for P-MEASURE): `just bench`. `crates/blaise-core/benches/route.rs` reads `bench.gtfs` from the root of the repository, a network compiled by gtfs-bin from a Stockholm feed (SL). It plans a `short` and a `long` route, forward and backward. Run them before and after a change made for speed, and put the numbers in the commit body.

## Hot paths

A route query is the hot path. It allocates nothing that outlives the query except the itinerary. In order of cost:

- `raptor::solve` and its rounds
- `Explorer::trip_patterns` and `Explorer::transfers`, which run on the rayon pool
- `State::begin_round`, `State::apply_updates` and `State::record_targets`
- `ParetoFront::is_dominated` and `ParetoFront::add`
- `SpatialGrid::iter_stops_in_radius`

## Known performance opportunities

Measured, not yet changed. A review doesn't repeat these. It reports one only if a change makes it worse, or notes when a change fixes one.

| Where | What | Numbers |
|---|---|---|
| `Explorer::trip_patterns`, `Explorer::transfers` | Each round returns a new `Vec<Update>`, and each rayon split starts its own. Reusing buffers kept in thread-locals is slower, and wrong when two searches share the pool. | Forward 5.9 ms to 6.8 ms, backward 11.7 ms to 14.1 ms (SL network) |
| `State::begin_round`, `Explorer::transfers` | A round clears and scans per-stop arrays whatever the number of marked stops. A list of marked stops would make a round cost what it marks. | not measured |

## Overrides

- **S-ERRORS (anyhow):** `blaise-core` is a library and returns `blaise_core::Error`. Only the `blaise` binary crate, `crates/blaise-cli`, uses `anyhow`.
