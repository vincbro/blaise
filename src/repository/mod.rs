mod models;
pub mod source;

pub use models::*;
use rayon::prelude::*;
use std::{collections::HashMap, sync::Arc};

use crate::{
    raptor::{Raptor, location::Location},
    shared::{
        self,
        geo::{AVERAGE_STOP_DISTANCE, Coordinate, Distance},
    },
};

// Global Urban Standard
type IdToIndex = HashMap<Arc<str>, usize>;
type IdToIndexes = HashMap<Arc<str>, Box<[usize]>>;
type IdToId = HashMap<Arc<str>, Arc<str>>;
type IdToIds = HashMap<Arc<str>, Box<[Arc<str>]>>;
type CellToIds = HashMap<(i32, i32), Box<[Arc<str>]>>;

#[derive(Debug, Clone, Default)]
pub struct Repository {
    pub stops: Box<[Stop]>,
    pub areas: Box<[Area]>,
    pub routes: Box<[Route]>,
    pub raptor_routes: Box<[RaptorRoute]>,
    pub trips: Box<[Trip]>,
    pub stop_times: Box<[StopTime]>,
    pub transfers: Box<[Transfer]>,

    // GTFS lookup
    stop_lookup: Arc<IdToIndex>,
    stop_distance_lookup: Arc<CellToIds>,
    area_lookup: Arc<IdToIndex>,
    route_lookup: Arc<IdToIndex>,
    route_to_trips: Arc<IdToIds>,
    trip_to_route: Arc<IdToId>,
    area_to_stops: Arc<IdToIds>,
    stop_to_area: Arc<IdToId>,
    stop_to_transfers: Arc<IdToIndexes>,
    stop_to_trips: Arc<IdToIds>,
    trip_lookup: Arc<IdToIndex>,
    trip_to_stop_times: Arc<IdToIndexes>,
    // Raptor lookup
    route_to_raptors: Arc<IdToIndexes>,
    stop_to_raptors: Arc<IdToIndexes>,
}

impl Repository {
    pub fn new() -> Self {
        Default::default()
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

    pub fn route_by_id(&self, id: &str) -> Option<&Route> {
        let index = self.route_lookup.get(id)?;
        Some(&self.routes[*index])
    }

    pub fn route_by_trip_id(&self, trip_id: &str) -> Option<&Route> {
        let id = self.trip_to_route.get(trip_id)?;
        self.route_by_id(id)
    }

    pub fn trips_by_route_id(&self, route_id: &str) -> Option<Vec<&Trip>> {
        let trips: Vec<_> = self
            .route_to_trips
            .get(route_id)?
            .iter()
            .filter_map(|trip_id| self.trip_by_id(trip_id))
            .collect();
        Some(trips)
    }

    pub fn stop_times_by_route_id(&self, route_id: &str) -> Option<Vec<Vec<&StopTime>>> {
        let trips = self.trips_by_route_id(route_id)?;
        let stop_times: Vec<_> = trips
            .into_par_iter()
            .filter_map(|trip| self.stop_times_by_trip_id(&trip.id))
            .collect();
        Some(stop_times)
    }

    pub fn routes_by_stop_id(&self, stop_id: &str) -> Option<Vec<&Route>> {
        self.trips_by_stop_id(stop_id)?
            .into_par_iter()
            .map(|trip| self.route_by_trip_id(&trip.id))
            .collect()
    }

    pub fn raptors_by_route_id(&self, route_id: &str) -> Option<Vec<&RaptorRoute>> {
        Some(
            self.route_to_raptors
                .get(route_id)?
                .iter()
                .map(|raptor_idx| &self.raptor_routes[*raptor_idx])
                .collect(),
        )
    }

    pub fn raptors_by_stop_id(&self, stop_id: &str) -> Option<Vec<&RaptorRoute>> {
        Some(
            self.stop_to_raptors
                .get(stop_id)?
                .iter()
                .map(|raptor_idx| &self.raptor_routes[*raptor_idx])
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
        let (origin_x, origin_y) = coordinate.to_grid();
        (-reach..=reach)
            .into_par_iter()
            .flat_map(|x| {
                (-reach..=reach)
                    .flat_map(move |y| {
                        let cell = (origin_x + x, origin_y + y);
                        if let Some(stop_ids) = self.stop_distance_lookup.get(&cell) {
                            stop_ids
                                .iter()
                                .filter_map(|stop_id| {
                                    self.stop_by_id(stop_id).filter(|stop| {
                                        stop.coordinate.network_distance(coordinate) <= distance
                                    })
                                })
                                .collect::<Vec<_>>()
                        } else {
                            Vec::new()
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Returns areas near there within the coordinates.
    pub fn areas_by_coordinate(&self, coordinate: &Coordinate, distance: Distance) -> Vec<&Area> {
        let stops = self.stops_by_coordinate(coordinate, distance);
        let mut areas: Vec<_> = stops
            .into_par_iter()
            .filter_map(|stop| self.area_by_stop_id(&stop.id))
            .collect();
        areas.par_sort_by_key(|area| area.index);
        areas.dedup_by_key(|area| area.index);
        areas
    }

    /// Does a fuzzy search on all the areas, comparing there name to the needle.
    pub fn search_areas_by_name<'a>(&'a self, needle: &'a str) -> Vec<&'a Area> {
        shared::search(needle, &self.areas)
    }

    /// Does a fuzzy search on all the stops, comparing there name to the needle.
    pub fn search_stops_by_name<'a>(&'a self, needle: &'a str) -> Vec<&'a Stop> {
        shared::search(needle, &self.stops)
    }

    pub fn router(&'_ self, from: Location, to: Location) -> Raptor<'_> {
        Raptor::new(self, from, to)
    }
}
