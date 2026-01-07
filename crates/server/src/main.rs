mod api;
mod dto;
mod state;

use crate::state::AppState;
use axum::routing::get;
use blaise::{gtfs::Gtfs, repository::Repository};
use std::{env, path::Path, process, sync::Arc, time::Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

const PORT: u32 = 3000;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let start_logo = include_str!("../start_logo.txt");
    println!("{}", start_logo);

    let gtfs_data_path = match env::var("GTFS_DATA_PATH") {
        Ok(path_str) => Path::new(&path_str).to_owned(),
        Err(err) => {
            error!("Failed loading GTFS_DATA_PATH: {}", err);
            process::exit(1);
        }
    };
    let app_state = AppState {
        repository: RwLock::new(None),
        gtfs_data_path,
    };

    if app_state.gtfs_data_path.exists() {
        info!("Loading data...");
        let now = Instant::now();
        let data = Gtfs::new().from_zip(&app_state.gtfs_data_path).unwrap();
        let repo = Repository::new().load_gtfs(data).unwrap();
        let _ = app_state.repository.write().await.replace(repo);
        info!("Loading data took {:?}", now.elapsed());
    } else {
        warn!("No GTFS data found.");
    }

    info!("Starting server...");

    let app = axum::Router::new()
        .route("/search", get(api::search))
        .route("/near", get(api::near))
        .route("/routing", get(api::routing))
        .route("/gtfs/fetch-url", get(api::fetch_url))
        .route("/gtfs/age", get(api::age))
        .with_state(Arc::new(app_state));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", PORT))
        .await
        .unwrap();
    info!("Listening to port {PORT}");
    axum::serve(listener, app).await.unwrap();
}
