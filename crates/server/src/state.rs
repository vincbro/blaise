use ontrack::repository::Repository;

pub struct AppState {
    pub repository: Repository,
}

impl AppState {
    pub fn new(repo: Repository) -> Self {
        Self { repository: repo }
    }
}
