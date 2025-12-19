use std::{collections::HashMap, sync::Arc};

mod models;
pub use models::*;

use crate::{
    gtfs,
    shared::{
        self,
        geo::{AVERAGE_STOP_DISTANCE, Coordinate, Distance},
        time::Duration,
    },
};

// Global Urban Standard
type IdToIndex = HashMap<Arc<str>, usize>;
type IdToIndexes = HashMap<Arc<str>, Arc<[usize]>>;
type IdToId = HashMap<Arc<str>, Arc<str>>;
type IdToIds = HashMap<Arc<str>, Arc<[Arc<str>]>>;
type CellToIds = HashMap<(i32, i32), Arc<[Arc<str>]>>;

#[derive(Debug, Clone, Default)]
pub struct Repository {
    pub(crate) stops: Arc<[Stop]>,
    pub(crate) areas: Arc<[Area]>,
    pub(crate) trips: Arc<[Trip]>,
    pub(crate) stop_times: Arc<[StopTime]>,
    pub(crate) transfers: Arc<[Transfer]>,

    // Lookup tables
    stop_lookup: Arc<IdToIndex>,
    stop_distance_lookup: Arc<CellToIds>,
    area_lookup: Arc<IdToIndex>,
    area_to_stops: Arc<IdToIds>,
    stop_to_area: Arc<IdToId>,
    stop_to_transfers: Arc<IdToIndexes>,
    stop_to_trips: Arc<IdToIds>,
    trip_lookup: Arc<IdToIndex>,
    trip_to_stop_times: Arc<IdToIndexes>,
}

impl Repository {
    pub fn new() -> Self {
        Default::default()
    }

    /// Used to stream data gtfs data into the engine
    /// Depending on the size of the data this can be a long blocking function
    pub fn with_gtfs(mut self, mut gtfs: gtfs::Gtfs) -> Result<Self, gtfs::Error> {
        // Build stop data set
        print!("Loading stops...");
        let mut stop_lookup: IdToIndex = HashMap::new();
        let mut stops: Vec<Stop> = Vec::new();
        gtfs.stream_stops(|(i, stop)| {
            let mut value: Stop = stop.into();
            value.index = i as u32;
            stop_lookup.insert(value.id.clone(), i);
            stops.push(value);
        })?;
        self.stops = stops.into();
        self.stop_lookup = stop_lookup.into();
        println!("OK");

        // Build area data set
        print!("Loading area...");
        let mut area_lookup: IdToIndex = HashMap::new();
        let mut areas: Vec<Area> = Vec::new();
        gtfs.stream_areas(|(i, area)| {
            let mut value: Area = area.into();
            value.index = i as u32;
            area_lookup.insert(value.id.clone(), i);
            areas.push(value);
        })?;
        self.areas = areas.into();
        self.area_lookup = area_lookup.into();
        println!("OK");

        // Build stop_area data set
        print!("Loading area to stop...");
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
        println!("OK");

        // Build trip data set
        print!("Loading trips...");
        let mut trip_lookup: IdToIndex = HashMap::new();
        let mut trips: Vec<Trip> = Vec::new();
        gtfs.stream_trips(|(i, trip)| {
            let mut value: Trip = trip.into();
            value.index = i as u32;
            trip_lookup.insert(value.id.clone(), i);
            trips.push(value);
        })?;
        self.trips = trips.into();
        self.trip_lookup = trip_lookup.into();
        println!("OK");

        // Loading in transfers
        print!("Loading transfers...");
        let mut transfers: Vec<Transfer> = Vec::new();
        let mut stop_to_transfers: HashMap<Arc<str>, Vec<usize>> = HashMap::new();
        gtfs.stream_transfers(|(i, transfer)| {
            let from_stop_idx = *self
                .stop_lookup
                .get(transfer.from_stop_id.as_str())
                .unwrap();
            let from_stop = &self.stops[from_stop_idx];

            let to_stop_idx = *self.stop_lookup.get(transfer.to_stop_id.as_str()).unwrap();
            let to_stop = &self.stops[to_stop_idx];

            let (from_trip_id, from_trip_idx) = if let Some(trip_id) = transfer.from_trip_id {
                let trip_idx = *self.trip_lookup.get(trip_id.as_str()).unwrap();
                let trip_id = self.trips[trip_idx].id.clone();
                (Some(trip_id), Some(trip_idx as u32))
            } else {
                (None, None)
            };

            let (to_trip_id, to_trip_idx) = if let Some(trip_id) = transfer.to_trip_id {
                let trip_idx = *self.trip_lookup.get(trip_id.as_str()).unwrap();
                let trip_id = self.trips[trip_idx].id.clone();
                (Some(trip_id), Some(trip_idx as u32))
            } else {
                (None, None)
            };

            stop_to_transfers
                .entry(from_stop.id.clone())
                .or_default()
                .push(i);

            let value = Transfer {
                from_stop_id: from_stop.id.clone(),
                from_stop_idx: from_stop_idx as u32,
                to_stop_id: to_stop.id.clone(),
                to_stop_idx: to_stop_idx as u32,
                from_trip_id,
                from_trip_idx,
                to_trip_id,
                to_trip_idx,
                min_transfer_time: transfer.min_transfer_time.map(Duration::from_seconds),
            };

            transfers.push(value);
        })?;
        self.transfers = transfers.into();
        let stop_to_transfers: IdToIndexes = stop_to_transfers
            .into_iter()
            .map(|(key, value)| (key, value.into()))
            .collect();
        self.stop_to_transfers = stop_to_transfers.into();
        println!("OK");

        // Build stop_time data set
        print!("Loading stop times...");
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
            value.trip_idx = *trip_index as u32;
            value.stop_id = stop.id.clone();
            value.stop_idx = *stop_index as u32;
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
        println!("OK");

        // Link area->stop->real world stop (stops that are linked to any trip)
        // This has to be last because it ties togheter alot
        // To save space and not having a O(n^2) operation trying to map each stop
        // to its nearby stops, we are going to map each stop with trips into a grid
        print!("Building distance grid...");
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
        println!("OK");

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

    pub fn coordinate_by_area_id(&self, area_id: &str) -> Option<Coordinate> {
        Some(
            self.stops_by_area_id(area_id)?
                .iter()
                .map(|stop| stop.coordinate)
                .sum(),
        )
    }

    /// Get all the possible transfers from a stop
    pub fn transfers_by_stop_id(&self, stop_id: &str) -> Option<Vec<&Transfer>> {
        let transfers = self.stop_to_transfers.get(stop_id)?;
        Some(
            transfers
                .iter()
                .map(|index| &self.transfers[*index])
                .collect(),
        )
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
                            && stop.coordinate.network_distance(coordinate) <= distance
                        {
                            stops.push(stop);
                        }
                    });
                }
            }
        }
        stops
    }

    /// Returns areas near there within the coordinates.
    pub fn areas_by_coordinate(&self, coordinate: &Coordinate, distance: Distance) -> Vec<&Area> {
        let stops = self.stops_by_coordinate(coordinate, distance);
        let mut areas: HashMap<&str, &Area> = HashMap::with_capacity(stops.len());
        stops
            .into_iter()
            .filter_map(|stop| self.area_by_stop_id(&stop.id))
            .for_each(|area| {
                areas.insert(&area.id, area);
            });
        areas.into_values().collect()
    }

    /// Does a fuzzy search on all the areas, comparing there name to the needle.
    pub fn search_areas_by_name<'a>(&'a self, needle: &'a str) -> Vec<&'a Area> {
        shared::search(needle, &self.areas)
    }

    /// Does a fuzzy search on all the stops, comparing there name to the needle.
    pub fn search_stops_by_name<'a>(&'a self, needle: &'a str) -> Vec<&'a Stop> {
        shared::search(needle, &self.stops)
    }

    // pub fn router(&self, from: Location, to: Location) -> Result<Router, routing::Error> {
    //     Router::new(self.clone(), from, to)
    // }
}
