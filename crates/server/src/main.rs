use crate::state::AppState;
use axum::routing::get;
use ontrack::{gtfs::Gtfs, repository::Repository};
use std::sync::Arc;
mod api;
mod dto;
mod state;

#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        println!("Missing gtfs zip");
        std::process::exit(1);
    }
    let path = std::path::Path::new(&args[1]).canonicalize().unwrap();

    let data = Gtfs::new(ontrack::gtfs::Config::default())
        .from_zip(path)
        .unwrap();
    let repo = Repository::new().with_gtfs(data).unwrap();

    // println!("CHECKING RAPTOR ROUTES");
    // repo.routes.iter().for_each(|route| {
    //     repo.raptors_by_route_id(&route.id)
    //         .unwrap()
    //         .into_iter()
    //         .for_each(|r| {
    //             let trips: Vec<_> = r
    //                 .trips
    //                 .iter()
    //                 .map(|trip_idx| &repo.trips[*trip_idx as usize])
    //                 .collect();
    //             for trip_a in trips.iter() {
    //                 let st_a = repo.stop_times_by_trip_id(&trip_a.id).unwrap();
    //                 for trip_b in trips.iter() {
    //                     let st_b = repo.stop_times_by_trip_id(&trip_b.id).unwrap();
    //                     if st_a.len() != st_b.len() {
    //                         println!("NOT THE SAME SIZE TRIPS!");
    //                     }

    //                     for i in 0..st_a.len() {
    //                         if st_a[i].stop_idx != st_b[i].stop_idx {
    //                             panic!("SEQUENCE NOT MATCHING");
    //                         }
    //                     }
    //                 }
    //             }
    //         });
    // });
    // println!("CLEAN!");

    let state = Arc::new(AppState::new(repo));

    let app = axum::Router::new()
        .route("/search", get(api::search))
        .route("/near", get(api::near))
        .route("/routing", get(api::routing))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
