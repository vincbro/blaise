use blaise::repository::Repository;
use std::path::PathBuf;
use tokio::sync::RwLock;

pub struct AppState {
    pub gtfs_data_path: PathBuf,
    pub repository: RwLock<Option<Repository>>,
}
