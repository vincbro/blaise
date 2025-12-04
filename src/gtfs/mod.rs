use serde::de::DeserializeOwned;
use std::{
    fs::File,
    io::{self},
    path::Path,
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
    Zip(ZipArchive<File>),
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

    pub fn from_zip<P: AsRef<Path>>(mut self, path: P) -> Result<Self, self::Error> {
        let zip_file = File::open(path)?;
        let archive = ZipArchive::new(zip_file)?;
        self.storage = StorageType::Zip(archive);
        Ok(self)
    }

    pub fn stream_stops<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsStop)),
    {
        match &mut self.storage {
            StorageType::None => Ok(()),
            StorageType::Zip(archive) => {
                stream_from_zip::<GtfsStop, F>(archive, &self.config.stops_file_name, f)
            }
        }
    }

    pub fn stream_areas<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsArea)),
    {
        match &mut self.storage {
            StorageType::None => Ok(()),
            StorageType::Zip(archive) => {
                stream_from_zip::<GtfsArea, F>(archive, &self.config.areas_file_name, f)
            }
        }
    }

    pub fn stream_stop_areas<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsStopArea)),
    {
        match &mut self.storage {
            StorageType::None => Ok(()),
            StorageType::Zip(archive) => {
                stream_from_zip::<GtfsStopArea, F>(archive, &self.config.stop_areas_file_name, f)
            }
        }
    }

    pub fn stream_stop_times<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsStopTime)),
    {
        match &mut self.storage {
            StorageType::None => Ok(()),
            StorageType::Zip(archive) => {
                stream_from_zip::<GtfsStopTime, F>(archive, &self.config.stop_times_file_name, f)
            }
        }
    }
}

fn stream_from_zip<T, F>(
    archive: &mut ZipArchive<File>,
    file_name: &str,
    f: F,
) -> Result<(), self::Error>
where
    T: DeserializeOwned,
    F: FnMut((usize, T)),
{
    let file = get_file(archive, file_name)?;
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
