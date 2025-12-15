use crate::state::AppState;
use axum::routing::get;
use std::sync::Arc;
mod api;
mod state;

#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        println!("Missing gtfs zip");
        std::process::exit(1);
    }
    let path = std::path::Path::new(&args[1]).canonicalize().unwrap();

    println!("Loading GTFS data");
    let data = ontrack::gtfs::Gtfs::new(ontrack::gtfs::Config::default())
        .from_zip(path)
        .unwrap();
    let engine = ontrack::engine::Engine::new().with_gtfs(data).unwrap();
    println!("Loading done");

    let state = Arc::new(AppState::new(engine));

    let app = axum::Router::new()
        .route("/search", get(api::search))
        .route("/routing", get(api::routing))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
