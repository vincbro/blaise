use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::{collections::HashMap, sync::Arc};

pub mod fuzzy;

mod area;
mod stop;
mod stop_time;
mod trip;
pub use area::*;
pub use stop::*;
pub use stop_time::*;
pub use trip::*;

use crate::gtfs::{self, Gtfs};

pub trait Identifiable {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn normalized_name(&self) -> &str;
}

#[derive(Clone, Default)]
pub struct Engine {
    stops: Arc<[Stop]>,
    areas: Arc<[Area]>,
    trips: Arc<[Trip]>,
    stop_times: Arc<[StopTime]>,

    // Lookup tables
    stop_lookup: Arc<HashMap<Arc<str>, usize>>,
    area_lookup: Arc<HashMap<Arc<str>, usize>>,
    trip_lookup: Arc<HashMap<Arc<str>, usize>>,
    area_to_stops: Arc<HashMap<Arc<str>, Vec<Arc<str>>>>,
    stop_to_area: Arc<HashMap<Arc<str>, Arc<str>>>,
}

impl Engine {
    pub fn new() -> Self {
        Default::default()
    }

    /// Used to stream data gtfs data into the engine
    /// Depending on the size of the data this can be a long blocking function
    pub fn with_gtfs(mut self, mut gtfs: Gtfs) -> Result<Self, gtfs::Error> {
        // Build stop data set
        let mut stop_lookup: HashMap<Arc<str>, usize> = HashMap::new();
        let mut stops: Vec<Stop> = Vec::new();
        gtfs.stream_stops(|(i, stop)| {
            let value: Stop = stop.into();
            stop_lookup.insert(value.id.clone(), i);
            stops.push(value);
        })?;
        self.stops = stops.into();
        self.stop_lookup = stop_lookup.into();

        // Build area data set
        let mut area_lookup: HashMap<Arc<str>, usize> = HashMap::new();
        let mut areas: Vec<Area> = Vec::new();
        gtfs.stream_areas(|(i, area)| {
            let value: Area = area.into();
            area_lookup.insert(value.id.clone(), i);
            areas.push(value);
        })?;
        self.areas = areas.into();
        self.area_lookup = area_lookup.into();

        // Build trip data set
        let mut trip_lookup: HashMap<Arc<str>, usize> = HashMap::new();
        let mut trips: Vec<Trip> = Vec::new();
        gtfs.stream_trips(|(i, trip)| {
            let value: Trip = trip.into();
            trip_lookup.insert(value.id.clone(), i);
            trips.push(value);
        })?;
        self.trips = trips.into();
        self.trip_lookup = trip_lookup.into();

        // Build stop_time data set
        let mut stop_times: Vec<StopTime> = Vec::new();
        gtfs.stream_stop_times(|(_, stop_time)| {
            let value: StopTime = stop_time.into();
            stop_times.push(value);
        })?;
        self.stop_times = stop_times.into();

        // Build stop_area data set
        let mut area_to_stops: HashMap<Arc<str>, Vec<Arc<str>>> = HashMap::new();
        let mut stop_to_area: HashMap<Arc<str>, Arc<str>> = HashMap::new();
        gtfs.stream_stop_areas(|(_, value)| {
            let stop_index = self.stop_lookup.get(value.stop_id.as_str()).unwrap();
            let stop_id = self.stops[*stop_index].id.clone();
            let area_index = self.area_lookup.get(value.area_id.as_str()).unwrap();
            let area_id = self.areas[*area_index].id.clone();

            stop_to_area.insert(stop_id.clone(), area_id.clone());
            if let Some(stops) = area_to_stops.get_mut(&area_id) {
                stops.push(stop_id);
            } else {
                area_to_stops.insert(area_id, vec![stop_id]);
            }
        })?;
        self.stop_to_area = stop_to_area.into();
        self.area_to_stops = area_to_stops.into();

        Ok(self)
    }

    pub fn area_by_id(&self, id: &str) -> Option<&Area> {
        let area_index = self.area_lookup.get(id)?;
        Some(&self.areas[*area_index])
    }

    pub fn stop_by_id(&self, id: &str) -> Option<&Stop> {
        let stop_index = self.stop_lookup.get(id)?;
        Some(&self.stops[*stop_index])
    }

    pub fn stops_by_area_id(&self, area_id: &str) -> Option<Vec<&Stop>> {
        let stops = self.area_to_stops.get(area_id)?;
        Some(
            stops
                .iter()
                .filter_map(|stop_id| self.stop_by_id(stop_id))
                .collect(),
        )
    }

    pub fn area_by_stop_id(&self, stop_id: &str) -> Option<&Area> {
        let area_id = self.stop_to_area.get(stop_id)?;
        self.area_by_id(area_id)
    }

    pub fn search_areas_by_name<'a>(&'a self, needle: &'a str) -> Vec<&'a Area> {
        search(needle, &self.areas)
    }

    pub fn search_stops_by_name<'a>(&'a self, needle: &'a str) -> Vec<&'a Stop> {
        search(needle, &self.stops)
    }
}

fn search<'a, T>(needle: &'a str, haystack: &'a [T]) -> Vec<&'a T>
where
    T: Send + Sync + Identifiable,
{
    let normalized_needle = needle.to_lowercase();
    let threads = rayon::current_num_threads();
    let chunk_size = haystack.len().div_ceil(threads);
    let mut results: Vec<Vec<(&T, f64)>> = Vec::with_capacity(threads);
    for _ in 0..threads {
        results.push(Vec::with_capacity(chunk_size));
    }
    results.par_iter_mut().enumerate().for_each(|(chunk, vec)| {
        for i in 0..chunk_size {
            let index = (chunk * chunk_size) + i;
            if index > haystack.len() - 1 {
                break;
            }
            let hay = &haystack[index];
            let score = fuzzy::score(&normalized_needle, hay.normalized_name());
            vec.push((hay, score));
        }
    });
    let mut results: Vec<_> = results.into_iter().flatten().collect();
    results.sort_by(|(_, score_a), (_, score_b)| score_b.total_cmp(score_a));
    results.iter().map(|(entity, _)| *entity).collect()
}
