use std::result;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no route was found")]
    NoRoute,
}

pub type Result<T> = result::Result<T, Error>;
