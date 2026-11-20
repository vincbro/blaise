//! How long a route query takes on a real network.
//!
//! It plans the same journeys a client would, forward from a departure and backward from an
//! arrival, with the `State` and `SpatialGrid` reused between queries. Each
//! phase is its own group with one entry per route, and reports the time of a single query, the
//! reset of the state before it included. Opening the network and building the grid are outside
//! the timer.
//!
//! The network is the file `bench.gtfs` in the root of the repository, one compiled by gtfs-bin
//! from a Stockholm feed: `gtfs-bin -i sweden.zip -o bench.gtfs`.

use std::{fs::File, hint};

use blaise_core::{
    raptor::{
        query::{QueryLocation, RaptorQuery},
        state::State,
    },
    spatial::SpatialGrid,
};
use criterion::{BenchmarkId, Criterion};
use gtfs_bin_core::{
    consumer::Consumer,
    models::{Coordinate, Time},
};
use memmap2::Mmap;

/// The network to plan on. Cargo runs a bench from its crate's directory, so the path starts
/// there.
const NETWORK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../bench.gtfs");

/// A journey as `(name, from, to)`, with positions as `(latitude, longitude)`.
type Route = (&'static str, (f64, f64), (f64, f64));

/// Within Stockholm, and from Södertälje to Uppsala.
const ROUTES: [Route; 2] = [
    ("short", (59.370_136, 18.001_749), (59.335_34, 18.057_737)),
    ("long", (59.196_198, 17.628_841), (59.857_834, 17.629_814)),
];

/// Leaves at 08:00, or arrives by 10:00.
const DEPARTURE: &str = "08:00:00";
const ARRIVAL: &str = "10:00:00";

/// Everything a query needs, built once.
struct Setup<'a> {
    consumer: &'a Consumer<'a>,
    spatial: SpatialGrid,
    state: State,
}

/// Plans one route and lets the result be dropped outside the timer.
fn plan(setup: &mut Setup<'_>, route: &Route, time: &str, backward: bool) {
    let time = Time::from_hms(time).expect("a valid time");
    let from = QueryLocation::Coordinate(Coordinate::new(route.1.0, route.1.1));
    let to = QueryLocation::Coordinate(Coordinate::new(route.2.0, route.2.1));
    let query = RaptorQuery::new(from, to);
    let query = if backward {
        query.with_arrival(time)
    } else {
        query.with_departure(time)
    };

    setup.state.reset();
    let itinerary = query.solve(setup.consumer, &setup.spatial, &mut setup.state);
    hint::black_box(&itinerary);
}

/// Times one query per route. A query that finds no route still takes the time it took to look.
fn phase(criterion: &mut Criterion, name: &str, time: &str, backward: bool) {
    let mmap = map();
    let consumer = Consumer::new(&mmap).expect("a valid network file");
    let mut setup = Setup {
        spatial: SpatialGrid::new(&consumer),
        state: State::new(&consumer),
        consumer: &consumer,
    };

    let mut group = criterion.benchmark_group(name);

    for route in &ROUTES {
        group.bench_with_input(
            BenchmarkId::from_parameter(route.0),
            route,
            |bencher, route| {
                bencher.iter(|| plan(&mut setup, route, time, backward));
            },
        );
    }

    group.finish();
}

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn map() -> Mmap {
    let file = File::open(NETWORK)
        .expect("bench.gtfs is in the root of the repository, see the top of this file");
    // SAFETY: the file is only read, and nothing here modifies it while it is mapped.
    unsafe { Mmap::map(&file) }.expect("the network file maps")
}

fn forward(criterion: &mut Criterion) {
    phase(criterion, "forward", DEPARTURE, false);
}

fn backward(criterion: &mut Criterion) {
    phase(criterion, "backward", ARRIVAL, true);
}

criterion::criterion_group!(benches, forward, backward);

criterion::criterion_main!(benches);
