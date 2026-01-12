use blaise::{
    gtfs::Gtfs,
    prelude::Repository,
    raptor::{self, Itinerary, Location},
    shared::{Coordinate, Time},
};
use criterion::{Criterion, criterion_group, criterion_main};
use std::{env, hint::black_box, path::Path, time::Duration};

fn short_solve(repository: &Repository) -> Result<Itinerary, raptor::Error> {
    let from: Location = Coordinate::from((59.370_136, 18.001_749)).into();
    let to: Location = Coordinate::from((59.335_34, 18.057_737)).into();
    let time = Time::from_seconds(28800);
    repository.router(from, to).departure_at(time).solve()
}

fn long_solve(repository: &Repository) -> Result<Itinerary, raptor::Error> {
    let from: Location = Coordinate::from((59.196_198, 17.628_841)).into();
    let to: Location = Coordinate::from((59.857_834, 17.629_814)).into();
    let time = Time::from_seconds(28800);
    repository.router(from, to).departure_at(time).solve()
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

    let mut group = c.benchmark_group("Routing");

    group.measurement_time(Duration::from_secs(30));

    group.bench_function("Short route solve", |b| {
        b.iter(|| black_box(short_solve(&repository)))
    });

    group.bench_function("Long route solve", |b| {
        b.iter(|| black_box(long_solve(&repository)))
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
