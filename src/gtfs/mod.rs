use memmap2::MmapOptions;
use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    slice::ParallelSlice,
};
use serde::de::DeserializeOwned;
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};
use thiserror::Error;
use tracing::info;

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
pub struct GtfsReader {
    config: Config,
    dir_path: PathBuf,
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
        let dir_path = GtfsReader::create_cache_dir(path)?;
        self.dir_path = dir_path;
        Ok(self)
    }

    pub fn from_zip_cache<P: AsRef<Path>>(mut self, path: P) -> Result<Self, self::Error> {
        let dir_path = GtfsReader::get_or_create_cache_dir(&path)?;
        self.dir_path = dir_path;
        Ok(self)
    }

    pub fn from_directory<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.dir_path = path.as_ref().to_path_buf();
        self
    }

    fn get_or_create_cache_dir<P: AsRef<Path>>(zip_path: P) -> Result<PathBuf, self::Error> {
        let zip_path = zip_path.as_ref();

        let mut target_dir = PathBuf::from(zip_path);
        target_dir.set_extension("");

        if !target_dir.exists() {
            target_dir = GtfsReader::create_cache_dir(zip_path)?;
        } else {
            info!("Using existing GTFS cache at {:?}", target_dir);
        }

        Ok(target_dir)
    }

    fn create_cache_dir<P: AsRef<Path>>(zip_path: P) -> Result<PathBuf, self::Error> {
        let zip_path = zip_path.as_ref();
        let mut target_dir = PathBuf::from(zip_path);
        target_dir.set_extension("");
        info!("Extracting GTFS to {:?}...", target_dir);
        fs::create_dir_all(&target_dir)?;
        let file = fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        archive.extract(&target_dir)?;
        Ok(target_dir)
    }

    /// Reads all the gtfs data into a `GtfsData` struct in parallel
    pub fn par_read(&self) -> Result<GtfsData, self::Error> {
        let config = &self.config;
        let file_names = config.to_slice();
        let dir_path = &self.dir_path;

        let data: Vec<GtfsTable> = file_names
            .into_par_iter()
            .map(|filename| {
                let file_path = dir_path.join(filename);

                if !file_path.exists() {
                    return Ok(GtfsTable::Unkown);
                }
                if filename == config.shapes_path.as_str() {
                    let data: Vec<GtfsShape> = GtfsReader::parse_big_file_parallel(file_path)?;
                    return Ok(GtfsTable::Shapes(data));
                } else if filename == config.stop_times_path.as_str() {
                    let data: Vec<GtfsStopTime> = GtfsReader::parse_big_file_parallel(file_path)?;
                    return Ok(GtfsTable::StopTimes(data));
                }

                let file = File::open(&file_path)?;
                let buf_reader = std::io::BufReader::with_capacity(128 * 1024, file);
                let mut reader = csv::Reader::from_reader(buf_reader);
                let table = if filename == config.stops_path.as_str() {
                    let data: Vec<GtfsStop> =
                        reader.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Stops(data)
                } else if filename == config.areas_path.as_str() {
                    let data: Vec<GtfsArea> =
                        reader.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Areas(data)
                } else if filename == config.stop_areas_path.as_str() {
                    let data: Vec<GtfsStopArea> =
                        reader.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::StopAreas(data)
                } else if filename == config.routes_path.as_str() {
                    let data: Vec<GtfsRoute> =
                        reader.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Routes(data)
                } else if filename == config.transfers_path.as_str() {
                    let data: Vec<GtfsTransfer> =
                        reader.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Transfers(data)
                } else if filename == config.trips_path.as_str() {
                    let data: Vec<GtfsTrip> =
                        reader.deserialize().collect::<Result<Vec<_>, _>>()?;
                    GtfsTable::Trips(data)
                } else {
                    GtfsTable::Unkown
                };
                Ok(table)
            })
            .collect::<Result<_, self::Error>>()?;
        Ok(GtfsData::from(data))
    }

    fn parse_big_file_parallel<P, T>(path: P) -> Result<Vec<T>, self::Error>
    where
        P: AsRef<Path>,
        T: DeserializeOwned + Sync + Send,
    {
        let file = File::open(path).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        let num_threads = rayon::current_num_threads();
        let len = mmap.len();
        let chunk_size = len / num_threads;

        let header_len = mmap.iter().position(|&b| b == b'\n').unwrap() + 1;
        let headers = &mmap[0..header_len];

        let mut offsets = Vec::with_capacity(num_threads + 1);
        offsets.push(header_len);

        for i in 1..num_threads {
            let mut target = i * chunk_size;
            while target < len && mmap[target] != b'\n' {
                target += 1;
            }
            offsets.push(target + 1);
        }
        offsets.push(len);

        let chunks: Result<Vec<Vec<T>>, csv::Error> = offsets
            .par_windows(2)
            .map(|window| {
                let start = window[0];
                let end = window[1];
                let slice = &mmap[start..end];
                let chain = headers.chain(slice);
                let mut csv = csv::ReaderBuilder::new()
                    .has_headers(true)
                    .from_reader(chain);

                csv.deserialize().collect::<Result<Vec<T>, csv::Error>>()
            })
            .collect();

        let data: Result<Vec<T>, csv::Error> = chunks.map(|c| c.into_iter().flatten().collect());
        Ok(data?)
    }
}
