use serde::de::DeserializeOwned;
use std::{
    fs::File,
    io::{self},
    path::PathBuf,
};
use thiserror::Error;
use zip::{ZipArchive, read::ZipFile};

mod config;
pub mod models;
pub use config::*;
use models::*;

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
    #[error("Could not find file with name: {0}")]
    FileNotFound(String),
}

#[derive(Default)]
pub enum StorageType {
    #[default]
    None,
    Zip(PathBuf),
}

#[derive(Default)]
pub struct Gtfs {
    config: Config,
    storage: StorageType,
}

impl Gtfs {
    pub fn new(config: self::Config) -> Self {
        Self {
            config,
            storage: Default::default(),
        }
    }

    pub fn from_zip(mut self, path: PathBuf) -> Self {
        self.storage = StorageType::Zip(path);
        self
    }

    pub fn stream_stops<F>(&self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsStop)),
    {
        match &self.storage {
            StorageType::None => Ok(()),
            StorageType::Zip(path) => {
                stream_from_zip::<GtfsStop, F>(path, &self.config.stops_file_name, f)
            }
        }
    }

    pub fn stream_areas<F>(&self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsArea)),
    {
        match &self.storage {
            StorageType::None => Ok(()),
            StorageType::Zip(path) => {
                stream_from_zip::<GtfsArea, F>(path, &self.config.areas_file_name, f)
            }
        }
    }

    pub fn stream_stop_areas<F>(&self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsStopArea)),
    {
        match &self.storage {
            StorageType::None => Ok(()),
            StorageType::Zip(path) => {
                stream_from_zip::<GtfsStopArea, F>(path, &self.config.stop_areas_file_name, f)
            }
        }
    }
}

fn stream_from_zip<T, F>(zip_path: &PathBuf, file_name: &str, f: F) -> Result<(), self::Error>
where
    T: DeserializeOwned,
    F: FnMut((usize, T)),
{
    let zip_file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(zip_file)?;
    let file = get_file(&mut archive, file_name)?;
    let mut reader = csv::Reader::from_reader(file);
    reader
        .deserialize()
        .filter_map(|a| a.ok())
        .enumerate()
        .for_each(f);
    Ok(())
}

fn get_file<'a>(
    archive: &'a mut ZipArchive<File>,
    name: &'a str,
) -> Result<ZipFile<'a, File>, self::Error> {
    let index = archive
        .index_for_name(name)
        .ok_or(self::Error::FileNotFound(name.to_string()))?;
    let file = archive.by_index(index)?;
    Ok(file)
}
