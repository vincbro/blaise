use ontrack::repository::Repository;

pub struct AppState {
    pub repo: Repository,
}

impl AppState {
    pub fn new(repo: Repository) -> Self {
        Self { repo }
    }
}
