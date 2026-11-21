use std::result;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no route was found")]
    NoRoute,
    #[error("a search over a time window starts from a departure time, not an arrival")]
    WindowNeedsDeparture,
}

pub type Result<T> = result::Result<T, Error>;
