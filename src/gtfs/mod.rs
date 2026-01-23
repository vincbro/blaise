mod config;
pub mod models;

pub use config::*;
use models::*;
use serde::de::DeserializeOwned;
use std::{
    fs::{self, File},
    io::{self},
    path::{Path, PathBuf},
};
use thiserror::Error;
use tracing::info;
use zip::{ZipArchive, read::ZipFile};

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
    #[error("Missing any source to pull data from")]
    MissingSource,
}

#[derive(Default)]
pub enum Source {
    #[default]
    None,
    Zip(ZipArchive<File>),
    Directory(PathBuf),
}

#[derive(Default)]
pub struct Gtfs {
    config: Config,
    storage: Source,
}

impl Gtfs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    pub fn from_zip<P: AsRef<Path>>(mut self, path: P) -> Result<Self, self::Error> {
        let zip_file = File::open(path)?;
        let archive = ZipArchive::new(zip_file)?;
        self.storage = Source::Zip(archive);
        Ok(self)
    }

    pub fn from_zip_cache<P: AsRef<Path>>(mut self, path: P) -> Result<Self, self::Error> {
        let directory = Gtfs::get_or_create_cache_dir(&path)?;
        self.storage = Source::Directory(directory);
        Ok(self)
    }

    pub fn from_directory<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.storage = Source::Directory(path.as_ref().to_path_buf());
        self
    }

    pub fn get_or_create_cache_dir<P: AsRef<Path>>(zip_path: P) -> Result<PathBuf, self::Error> {
        let zip_path = zip_path.as_ref();

        let mut target_dir = PathBuf::from(zip_path);
        target_dir.set_extension("");

        if !target_dir.exists() {
            info!("Extracting GTFS to {:?}...", target_dir);
            fs::create_dir_all(&target_dir)?;

            let file = fs::File::open(zip_path)?;
            let mut archive = zip::ZipArchive::new(file)?;
            archive.extract(&target_dir)?;
        } else {
            info!("Using existing GTFS cache at {:?}", target_dir);
        }

        Ok(target_dir)
    }

    pub fn stream_stops<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsStop)),
    {
        match &mut self.storage {
            Source::None => Err(self::Error::MissingSource),
            Source::Zip(archive) => stream_from_zip(archive, &self.config.stops_path, f),
            Source::Directory(path) => stream_from_dir(path, &self.config.stops_path, f),
        }
    }

    pub fn stream_areas<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsArea)),
    {
        match &mut self.storage {
            Source::None => Err(self::Error::MissingSource),
            Source::Zip(archive) => stream_from_zip(archive, &self.config.areas_path, f),
            Source::Directory(path) => stream_from_dir(path, &self.config.areas_path, f),
        }
    }

    pub fn stream_stop_areas<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsStopArea)),
    {
        match &mut self.storage {
            Source::None => Err(self::Error::MissingSource),
            Source::Zip(archive) => stream_from_zip(archive, &self.config.stop_areas_path, f),
            Source::Directory(path) => stream_from_dir(path, &self.config.stop_areas_path, f),
        }
    }

    pub fn stream_stop_times<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsStopTime)),
    {
        match &mut self.storage {
            Source::None => Err(self::Error::MissingSource),
            Source::Zip(archive) => stream_from_zip(archive, &self.config.stop_times_path, f),
            Source::Directory(path) => stream_from_dir(path, &self.config.stop_times_path, f),
        }
    }

    pub fn stream_transfers<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsTransfer)),
    {
        match &mut self.storage {
            Source::None => Err(self::Error::MissingSource),
            Source::Zip(archive) => stream_from_zip(archive, &self.config.transfers_path, f),
            Source::Directory(path) => stream_from_dir(path, &self.config.transfers_path, f),
        }
    }

    pub fn stream_routes<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsRoute)),
    {
        match &mut self.storage {
            Source::None => Err(self::Error::MissingSource),
            Source::Zip(archive) => stream_from_zip(archive, &self.config.routes_path, f),
            Source::Directory(path) => stream_from_dir(path, &self.config.routes_path, f),
        }
    }

    pub fn stream_trips<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsTrip)),
    {
        match &mut self.storage {
            Source::None => Ok(()),
            Source::Zip(archive) => stream_from_zip(archive, &self.config.trips_path, f),
            Source::Directory(path) => stream_from_dir(path, &self.config.trips_path, f),
        }
    }
}

fn stream_from_zip<T, F>(
    archive: &mut ZipArchive<File>,
    file_name: &str,
    mut f: F,
) -> Result<(), self::Error>
where
    T: DeserializeOwned,
    F: FnMut((usize, T)),
{
    let file = get_file_from_zip(archive, file_name)?;
    let mut reader = csv::Reader::from_reader(file);
    for (i, result) in reader.deserialize().enumerate() {
        let record: T = result?;
        f((i, record));
    }
    Ok(())
}

fn stream_from_dir<T, F>(dir_path: &Path, file_name: &str, mut f: F) -> Result<(), self::Error>
where
    T: serde::de::DeserializeOwned,
    F: FnMut((usize, T)),
{
    let file_path = dir_path.join(file_name);
    let file = fs::File::open(file_path)?;

    // BufReader is critical here for speed
    let reader = std::io::BufReader::with_capacity(128 * 1024, file);
    let mut csv_reader = csv::Reader::from_reader(reader);

    for (i, result) in csv_reader.deserialize().enumerate() {
        let record: T = result?;
        f((i, record));
    }
    Ok(())
}

fn get_file_from_zip<'a>(
    archive: &'a mut ZipArchive<File>,
    name: &'a str,
) -> Result<ZipFile<'a, File>, self::Error> {
    let index = archive
        .index_for_name(name)
        .ok_or(self::Error::FileNotFound(name.to_string()))?;
    let file = archive.by_index(index)?;
    Ok(file)
}
