mod api;
mod dto;
mod state;

use crate::state::AppState;
use axum::routing::get;
use ontrack::{gtfs::Gtfs, repository::Repository};
use std::{sync::Arc, time::Instant};
use tracing::{error, info};

const PORT: u32 = 3000;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let start_logo = include_str!("../start_logo.txt");
    println!("{}", start_logo);

    info!("Starting server...");
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        error!("Missing gtfs zip");
        std::process::exit(1);
    }
    let path = std::path::Path::new(&args[1]).canonicalize().unwrap();

    info!("Loading data...");
    let now = Instant::now();
    let data = Gtfs::new().from_zip(path).unwrap();
    let repo = Repository::new().load_gtfs(data).unwrap();
    let state = Arc::new(AppState::new(repo));
    info!("Loading data took {:?}", now.elapsed());

    let app = axum::Router::new()
        .route("/search", get(api::search))
        .route("/near", get(api::near))
        .route("/routing", get(api::routing))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", PORT))
        .await
        .unwrap();
    info!("Listening to port {PORT}");
    axum::serve(listener, app).await.unwrap();
}
