use std::path::PathBuf;

use ontrack::repository::Repository;
use tokio::sync::RwLock;

pub struct AppState {
    pub gtfs_data_path: PathBuf,
    pub repository: RwLock<Option<Repository>>,
}
