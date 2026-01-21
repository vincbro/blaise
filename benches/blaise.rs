use blaise::{
    gtfs::Gtfs,
    prelude::Repository,
    raptor::{Allocator, Location},
    shared::{AVERAGE_STOP_DISTANCE, Coordinate, Distance, Time},
};
use criterion::{Criterion, criterion_group, criterion_main};
use std::{env, hint::black_box, path::Path, time::Duration};

fn geo_lookup_tile_size(repository: &Repository) {
    let coordinate = Coordinate::from((59.370_136, 18.001_749));
    let _ = black_box(repository.stops_by_coordinate(&coordinate, AVERAGE_STOP_DISTANCE));
}

fn geo_lookup_10x_tile_size(repository: &Repository) {
    const DISTANCE: Distance = Distance::from_meters(AVERAGE_STOP_DISTANCE.as_meters() * 10.0);
    let coordinate = Coordinate::from((59.370_136, 18.001_749));
    let _ = black_box(repository.stops_by_coordinate(&coordinate, DISTANCE));
}

fn short_solve(repository: &Repository, allocator: &mut Allocator) {
    let from: Location = Coordinate::from((59.370_136, 18.001_749)).into();
    let to: Location = Coordinate::from((59.335_34, 18.057_737)).into();
    let time = Time::from_seconds(28800);
    allocator.reset();
    let _ = black_box(
        repository
            .router(from, to)
            .departure_at(time)
            .solve_with_allocator(allocator),
    );
}

fn long_solve(repository: &Repository, allocator: &mut Allocator) {
    let from: Location = Coordinate::from((59.196_198, 17.628_841)).into();
    let to: Location = Coordinate::from((59.857_834, 17.629_814)).into();
    let time = Time::from_seconds(28800);
    allocator.reset();
    let _ = black_box(
        repository
            .router(from, to)
            .departure_at(time)
            .solve_with_allocator(allocator),
    );
}
fn criterion_benchmark(c: &mut Criterion) {
    let gtfs_data_path = match env::var("GTFS_DATA_PATH") {
        Ok(path_str) => Path::new(&path_str).to_owned(),
        Err(err) => {
            println!("Missing GTFS_DATA_PATH environment variable: {err}");
            return;
        }
    };

    let gtfs = Gtfs::new()
        .from_zip(gtfs_data_path)
        .expect("Failed to load GTFS zip");
    let repository = Repository::new()
        .load_gtfs(gtfs)
        .expect("Failed to build repository");

    let mut allocator = Allocator::new(&repository);

    let mut group = c.benchmark_group("Routing");

    group.warm_up_time(Duration::from_secs(10));

    group.measurement_time(Duration::from_secs(30));

    // group.bench_function("Distance 1x", |b| {
    //     b.iter(|| geo_lookup_tile_size(&repository))
    // });

    // group.bench_function("Distance 10x", |b| {
    //     b.iter(|| geo_lookup_10x_tile_size(&repository))
    // });

    group.bench_function("Short route solve", |b| {
        b.iter(|| short_solve(&repository, &mut allocator))
    });

    group.bench_function("Long route solve", |b| {
        b.iter(|| long_solve(&repository, &mut allocator))
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
