use std::io;

use thiserror::Error;

mod loader;
mod models;

pub use loader::*;
pub use models::*;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Csv error: {0}")]
    Csv(#[from] csv::Error),
    #[error("Csv file {0} is missing header")]
    MissingHeader(String),
}
