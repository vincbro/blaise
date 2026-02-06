use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::de::DeserializeOwned;
use std::{
    fs::{self, File},
    io::{self},
    path::{Path, PathBuf},
};
use thiserror::Error;
use tracing::info;
use zip::{ZipArchive, read::ZipFile};

mod config;
mod data;
mod models;

pub use config::*;
pub use data::*;
pub use models::*;

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

#[derive(Default, Debug)]
pub enum Source {
    #[default]
    None,
    Zip(ZipArchive<File>),
    Directory(PathBuf),
}

#[derive(Default, Debug)]
pub struct GtfsReader {
    config: Config,
    storage: Source,
}

impl GtfsReader {
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
        let directory = GtfsReader::get_or_create_cache_dir(&path)?;
        self.storage = Source::Directory(directory);
        Ok(self)
    }

    pub fn from_directory<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.storage = Source::Directory(path.as_ref().to_path_buf());
        self
    }

    fn get_or_create_cache_dir<P: AsRef<Path>>(zip_path: P) -> Result<PathBuf, self::Error> {
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

    /// Reads all the gtfs data into a `GtfsData` struct in parallel
    pub fn par_read(&self) -> Result<GtfsData, self::Error> {
        let config = &self.config;
        let file_names = config.to_slice();

        let dir_path = match &self.storage {
            Source::Directory(p) => p,
            Source::Zip(_) => {
                return Err(self::Error::Io(std::io::Error::other(
                    "Parallel read requires Source::Directory (use from_zip_cache)",
                )));
            }
            Source::None => return Err(self::Error::MissingSource),
        };

        let data: Vec<GtfsTable> = file_names
            .into_par_iter()
            .map(|filename| {
                let file_path = dir_path.join(filename);

                if !file_path.exists() {
                    return Ok(GtfsTable::Unkown);
                }
                let file = File::open(&file_path)?;
                let reader = std::io::BufReader::with_capacity(128 * 1024, file);
                let mut csv = csv::Reader::from_reader(reader);
                let table = if filename == config.stops_path.as_str() {
                    let data: Vec<GtfsStop> = csv.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Stops(data)
                } else if filename == config.areas_path.as_str() {
                    let data: Vec<GtfsArea> = csv.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Areas(data)
                } else if filename == config.stop_areas_path.as_str() {
                    let data: Vec<GtfsStopArea> =
                        csv.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::StopAreas(data)
                } else if filename == config.routes_path.as_str() {
                    let data: Vec<GtfsRoute> = csv.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Routes(data)
                } else if filename == config.transfers_path.as_str() {
                    let data: Vec<GtfsTransfer> =
                        csv.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Transfers(data)
                } else if filename == config.stop_times_path.as_str() {
                    let data: Vec<GtfsStopTime> =
                        csv.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::StopTimes(data)
                } else if filename == config.shapes_path.as_str() {
                    let data: Vec<GtfsShape> = csv.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Shapes(data)
                } else if filename == config.trips_path.as_str() {
                    let data: Vec<GtfsTrip> = csv.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Trips(data)
                } else {
                    GtfsTable::Unkown
                };
                Ok(table)
            })
            .collect::<Result<_, self::Error>>()?;
        Ok(GtfsData::from(data))
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

    pub fn stream_shapes<F>(&mut self, f: F) -> Result<(), self::Error>
    where
        F: FnMut((usize, GtfsShape)),
    {
        match &mut self.storage {
            Source::None => Ok(()),
            Source::Zip(archive) => stream_from_zip(archive, &self.config.shapes_path, f),
            Source::Directory(path) => stream_from_dir(path, &self.config.shapes_path, f),
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
