use ontrack::engine::Engine;

pub struct AppState {
    pub engine: Engine,
}

impl AppState {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }
}
