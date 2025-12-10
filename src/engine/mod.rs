use std::{collections::HashMap, sync::Arc};

// Util
pub mod fuzzy;
pub mod geo;
pub mod routing;
pub mod search;

// Models
mod area;
mod stop;
mod stop_time;
mod trip;
pub use area::*;
pub use stop::*;
pub use stop_time::*;
pub use trip::*;

use crate::{
    engine::{
        geo::{Coordinate, Distance},
        routing::Router,
    },
    gtfs::{self, Gtfs},
};

// Global Urban Standard
pub(crate) const AVERAGE_STOP_DISTANCE: Distance = Distance::meters(500.0);
pub(crate) const LONGITUDE_DISTANCE: Distance = Distance::meters(111_320.0);
pub(crate) const LATITUDE_DISTANCE: Distance = Distance::meters(110_540.0);

pub trait Identifiable {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn normalized_name(&self) -> &str;
}

type IdToIndex = HashMap<Arc<str>, usize>;
type IdToIndexes = HashMap<Arc<str>, Arc<[usize]>>;
type IdToId = HashMap<Arc<str>, Arc<str>>;
type IdToIds = HashMap<Arc<str>, Arc<[Arc<str>]>>;
type CellToIds = HashMap<(i32, i32), Arc<[Arc<str>]>>;

#[derive(Debug, Clone, Default)]
pub struct Engine {
    stops: Arc<[Stop]>,
    areas: Arc<[Area]>,
    trips: Arc<[Trip]>,
    stop_times: Arc<[StopTime]>,

    // Lookup tables
    stop_lookup: Arc<IdToIndex>,
    stop_distance_lookup: Arc<CellToIds>,
    area_lookup: Arc<IdToIndex>,
    trip_lookup: Arc<IdToIndex>,
    trip_to_stop_times: Arc<IdToIndexes>,
    stop_to_trips: Arc<IdToIds>,
    area_to_stops: Arc<IdToIds>,
    stop_to_area: Arc<IdToId>,
}

impl Engine {
    pub fn new() -> Self {
        Default::default()
    }

    /// Used to stream data gtfs data into the engine
    /// Depending on the size of the data this can be a long blocking function
    pub fn with_gtfs(mut self, mut gtfs: Gtfs) -> Result<Self, gtfs::Error> {
        // Build stop data set
        let mut stop_lookup: IdToIndex = HashMap::new();
        let mut stops: Vec<Stop> = Vec::new();
        gtfs.stream_stops(|(i, stop)| {
            let value: Stop = stop.into();
            stop_lookup.insert(value.id.clone(), i);
            stops.push(value);
        })?;
        self.stops = stops.into();
        self.stop_lookup = stop_lookup.into();
        println!("Stops done");

        // Build area data set
        let mut area_lookup: IdToIndex = HashMap::new();
        let mut areas: Vec<Area> = Vec::new();
        gtfs.stream_areas(|(i, area)| {
            let value: Area = area.into();
            area_lookup.insert(value.id.clone(), i);
            areas.push(value);
        })?;
        self.areas = areas.into();
        self.area_lookup = area_lookup.into();
        println!("Areas done");

        // Build stop_area data set
        let mut area_to_stops: HashMap<Arc<str>, Vec<Arc<str>>> = HashMap::new();
        let mut stop_to_area: IdToId = HashMap::new();
        gtfs.stream_stop_areas(|(_, value)| {
            // TEMP
            let stop_index = self.stop_lookup.get(value.stop_id.as_str()).unwrap();
            let stop_id = self.stops[*stop_index].id.clone();
            // TEMP
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
        let area_to_stops: IdToIds = area_to_stops
            .into_iter()
            .map(|(key, value)| (key, value.into()))
            .collect();
        self.area_to_stops = area_to_stops.into();
        println!("Area to stops done");

        // Build trip data set
        let mut trip_lookup: IdToIndex = HashMap::new();
        let mut trips: Vec<Trip> = Vec::new();
        gtfs.stream_trips(|(i, trip)| {
            let value: Trip = trip.into();
            trip_lookup.insert(value.id.clone(), i);
            trips.push(value);
        })?;
        self.trips = trips.into();
        self.trip_lookup = trip_lookup.into();
        println!("Trips done");

        // Build stop_time data set
        let mut trip_to_stop_times: HashMap<Arc<str>, Vec<usize>> = HashMap::new();
        let mut stop_to_trips: HashMap<Arc<str>, Vec<Arc<str>>> = HashMap::new();
        let mut stop_times: Vec<StopTime> = Vec::new();
        gtfs.stream_stop_times(|(i, stop_time)| {
            // TEMP
            let trip_index = self.trip_lookup.get(stop_time.trip_id.as_str()).unwrap();
            let trip = &self.trips[*trip_index];

            // TEMP
            let stop_index = self.stop_lookup.get(stop_time.stop_id.as_str()).unwrap();
            let stop = &self.stops[*stop_index];

            let mut value: StopTime = stop_time.into();
            value.trip_id = trip.id.clone();
            stop_times.push(value);

            trip_to_stop_times
                .entry(trip.id.clone())
                .or_default()
                .push(i);
            stop_to_trips
                .entry(stop.id.clone())
                .or_default()
                .push(trip.id.clone());
        })?;
        self.stop_times = stop_times.into();
        let trip_to_stop_times: IdToIndexes = trip_to_stop_times
            .into_iter()
            .map(|(key, value)| (key, value.into()))
            .collect();
        self.trip_to_stop_times = trip_to_stop_times.into();

        let stop_to_trips: IdToIds = stop_to_trips
            .into_iter()
            .map(|(key, value)| (key, value.into()))
            .collect();
        self.stop_to_trips = stop_to_trips.into();
        println!("Stop times done");

        // Link area->stop->real world stop (stops that are linked to any trip)
        // This has to be last because it ties togheter alot
        // To save space and not having a O(n^2) operation trying to map each stop
        // to its nearby stops, we are going to map each stop with trips into a grid
        let mut stop_distance_lookup: HashMap<(i32, i32), Vec<Arc<str>>> = HashMap::new();
        self.stops
            .iter()
            // .filter(|stop| self.trips_by_stop_id(&stop.id).is_some())
            .for_each(|stop| {
                let cell = stop.coordinate.to_grid();
                stop_distance_lookup
                    .entry(cell)
                    .or_default()
                    .push(stop.id.clone());
            });
        let stop_distance_lookup: CellToIds = stop_distance_lookup
            .into_iter()
            .map(|(cell, stops)| (cell, stops.into()))
            .collect();
        self.stop_distance_lookup = stop_distance_lookup.into();
        println!("Area to stops to real world stops done");

        Ok(self)
    }

    /// Get an area with the given id.
    /// If no area is found with the given id None is returned.
    /// Area is safe and quick to clone if a owned instance is needed.
    pub fn area_by_id(&self, id: &str) -> Option<&Area> {
        let area_index = self.area_lookup.get(id)?;
        Some(&self.areas[*area_index])
    }

    /// Get an stop with the given id.
    /// If no stop is found with the given id None is returned.
    /// Stop is safe and quick to clone if a owned instance is needed.
    pub fn stop_by_id(&self, id: &str) -> Option<&Stop> {
        let stop_index = self.stop_lookup.get(id)?;
        Some(&self.stops[*stop_index])
    }

    /// Returns all the stops in an area with the given id.
    /// The if there is no area with the given id None is returned.
    /// Stop is safe and quick to clone if a owned instance is needed.
    pub fn stops_by_area_id(&self, area_id: &str) -> Option<Vec<&Stop>> {
        let stops = self.area_to_stops.get(area_id)?;
        Some(
            stops
                .iter()
                .filter_map(|stop_id| self.stop_by_id(stop_id))
                .collect(),
        )
    }

    /// Gets the area that the given stop is in.
    /// If no stop, or area is found None is returned.
    pub fn area_by_stop_id(&self, stop_id: &str) -> Option<&Area> {
        let area_id = self.stop_to_area.get(stop_id)?;
        self.area_by_id(area_id)
    }

    /// Gets a trip with the given id.
    /// If no trip with the given id was found None is returned.
    pub fn trip_by_id(&self, id: &str) -> Option<&Trip> {
        let trip_index = self.trip_lookup.get(id)?;
        Some(&self.trips[*trip_index])
    }

    /// Returns all the trips that go trough a given stop.
    /// If no stop was found with the given id none is returned.
    pub fn trips_by_stop_id(&self, stop_id: &str) -> Option<Vec<&Trip>> {
        let trips = self.stop_to_trips.get(stop_id)?;
        Some(
            trips
                .iter()
                .filter_map(|trip_id| self.trip_by_id(trip_id))
                .collect(),
        )
    }

    /// Returns all the stop times for a given trip.
    /// If no trip was found with the given id None is returned.
    pub fn stop_times_by_trip_id(&self, trip_id: &str) -> Option<Vec<&StopTime>> {
        let stop_times = self.trip_to_stop_times.get(trip_id)?;
        Some(stop_times.iter().map(|i| &self.stop_times[*i]).collect())
    }

    /// Returns stops near there within the coordinates.
    pub fn stops_by_coordinate(&self, coordinate: &Coordinate, distance: Distance) -> Vec<&Stop> {
        let reach = (distance / AVERAGE_STOP_DISTANCE).as_meters().ceil().abs() as i32;
        let cell = coordinate.to_grid();
        let mut stops: Vec<&Stop> = Vec::new();
        for x in -reach..reach + 1 {
            for y in -reach..reach + 1 {
                let cell = (cell.0 + x, cell.1 + y);
                if let Some(stop_ids) = self.stop_distance_lookup.get(&cell) {
                    stop_ids.iter().for_each(|stop_id| {
                        if let Some(stop) = self.stop_by_id(stop_id)
                            && stop.coordinate.distance(coordinate) <= distance
                        {
                            stops.push(stop);
                        }
                    });
                }
            }
        }
        stops
    }

    /// Does a fuzzy search on all the areas, comparing there name to the needle.
    pub fn search_areas_by_name<'a>(&'a self, needle: &'a str) -> Vec<&'a Area> {
        search::search(needle, &self.areas)
    }

    /// Does a fuzzy search on all the stops, comparing there name to the needle.
    pub fn search_stops_by_name<'a>(&'a self, needle: &'a str) -> Vec<&'a Stop> {
        search::search(needle, &self.stops)
    }

    pub fn router(&self) -> Router {
        Router::new(self.clone())
    }
}
