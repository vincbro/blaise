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
            StorageType::Zip(archive) => stream_from_zip(archive, &self.config.stops_path, f),
            _ => todo!(),
        }
    }

    pub fn stream_areas<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsArea)),
    {
        match &mut self.storage {
            StorageType::Zip(archive) => stream_from_zip(archive, &self.config.areas_path, f),
            _ => todo!(),
        }
    }

    pub fn stream_stop_areas<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsStopArea)),
    {
        match &mut self.storage {
            StorageType::Zip(archive) => stream_from_zip(archive, &self.config.stop_areas_path, f),
            _ => todo!(),
        }
    }

    pub fn stream_stop_times<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsStopTime)),
    {
        match &mut self.storage {
            StorageType::Zip(archive) => stream_from_zip(archive, &self.config.stop_times_path, f),
            _ => todo!(),
        }
    }

    pub fn stream_transfers<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsTransfer)),
    {
        match &mut self.storage {
            StorageType::None => Ok(()),
            StorageType::Zip(archive) => stream_from_zip(archive, &self.config.transfers_path, f),
        }
    }

    pub fn stream_routes<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsRoute)),
    {
        match &mut self.storage {
            StorageType::None => Ok(()),
            StorageType::Zip(archive) => stream_from_zip(archive, &self.config.routes_path, f),
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
