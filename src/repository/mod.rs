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
type IdToIndex = HashMap<Arc<str>, u32>;
type CellToIdx = HashMap<(i32, i32), Box<[u32]>>;

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
    stop_lookup: IdToIndex,
    trip_lookup: IdToIndex,
    area_lookup: IdToIndex,
    route_lookup: IdToIndex,
    stop_distance_lookup: CellToIdx,

    // Maps
    route_to_trips: Box<[Box<[u32]>]>,
    trip_to_route: Box<[u32]>,
    area_to_stops: Box<[Box<[u32]>]>,
    stop_to_area: Box<[u32]>,
    stop_to_transfers: Box<[Box<[u32]>]>,
    stop_to_trips: Box<[Box<[u32]>]>,
    trip_to_stop_slice: Box<[StopTimeSlice]>,
    // Raptor lookup
    route_to_raptors: Box<[Box<[u32]>]>,
    stop_to_raptors: Box<[Box<[u32]>]>,
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
        Some(&self.areas[*area_index as usize])
    }

    /// Get an stop with the given id.
    /// If no stop is found with the given id None is returned.
    /// Stop is safe and quick to clone if a owned instance is needed.
    pub fn stop_by_id(&self, id: &str) -> Option<&Stop> {
        let stop_index = self.stop_lookup.get(id)?;
        Some(&self.stops[*stop_index as usize])
    }

    /// Returns all the stops in an area with the given id.
    /// The if there is no area with the given id None is returned.
    /// Stop is safe and quick to clone if a owned instance is needed.
    pub fn stops_by_area_id(&self, area_id: &str) -> Option<Vec<&Stop>> {
        let area_idx = self.area_lookup.get(area_id)?;
        Some(self.stops_by_area_idx(*area_idx))
    }

    pub fn stops_by_area_idx(&self, area_idx: u32) -> Vec<&Stop> {
        self.area_to_stops[area_idx as usize]
            .iter()
            .map(|stop_idx| &self.stops[*stop_idx as usize])
            .collect()
    }

    /// Gets the area that the given stop is in.
    /// If no stop, or area is found None is returned.
    pub fn area_by_stop_id(&self, stop_id: &str) -> Option<&Area> {
        let stop_idx = self.stop_lookup.get(stop_id)?;
        Some(self.area_by_stop_idx(*stop_idx))
    }

    pub fn area_by_stop_idx(&self, stop_idx: u32) -> &Area {
        let area_idx = self.stop_to_area[stop_idx as usize];
        &self.areas[area_idx as usize]
    }

    pub fn coordinate_by_area_id(&self, area_id: &str) -> Option<Coordinate> {
        Some(
            self.stops_by_area_id(area_id)?
                .iter()
                .map(|stop| stop.coordinate)
                .sum(),
        )
    }

    pub fn coordinate_by_area_idx(&self, area_idx: u32) -> Coordinate {
        self.stops_by_area_idx(area_idx)
            .iter()
            .map(|stop| stop.coordinate)
            .sum()
    }

    /// Get all the possible transfers from a stop
    pub fn transfers_by_stop_id(&self, stop_id: &str) -> Option<Vec<&Transfer>> {
        let stop = self.stop_by_id(stop_id)?;
        Some(self.transfers_by_stop_idx(stop.index))
    }

    pub fn transfers_by_stop_idx(&self, stop_idx: u32) -> Vec<&Transfer> {
        let transfers = &self.stop_to_transfers[stop_idx as usize];
        transfers
            .iter()
            .map(|transfer_idx| &self.transfers[*transfer_idx as usize])
            .collect()
    }
    /// Gets a trip with the given id.
    /// If no trip with the given id was found None is returned.
    pub fn trip_by_id(&self, id: &str) -> Option<&Trip> {
        let trip_index = self.trip_lookup.get(id)?;
        Some(&self.trips[*trip_index as usize])
    }

    /// Returns all the trips that go trough a given stop.
    /// If no stop was found with the given id none is returned.
    pub fn trips_by_stop_id(&self, stop_id: &str) -> Option<Vec<&Trip>> {
        let stop_idx = self.stop_lookup.get(stop_id)?;
        Some(self.trips_by_stop_idx(*stop_idx))
    }

    pub fn trips_by_stop_idx(&self, stop_idx: u32) -> Vec<&Trip> {
        self.stop_to_trips[stop_idx as usize]
            .iter()
            .map(|trip_idx| &self.trips[*trip_idx as usize])
            .collect()
    }

    pub fn route_by_id(&self, id: &str) -> Option<&Route> {
        let index = self.route_lookup.get(id)?;
        Some(&self.routes[*index as usize])
    }

    pub fn route_by_trip_id(&self, trip_id: &str) -> Option<&Route> {
        let trip_idx = self.trip_lookup.get(trip_id)?;
        Some(self.route_by_trip_idx(*trip_idx))
    }

    pub fn route_by_trip_idx(&self, trip_idx: u32) -> &Route {
        let route_idx = self.trip_to_route[trip_idx as usize];
        &self.routes[route_idx as usize]
    }

    pub fn trips_by_route_id(&self, route_id: &str) -> Option<Vec<&Trip>> {
        let route_idx = self.route_lookup.get(route_id)?;
        Some(self.trips_by_route_idx(*route_idx))
    }

    pub fn trips_by_route_idx(&self, route_idx: u32) -> Vec<&Trip> {
        self.route_to_trips[route_idx as usize]
            .iter()
            .map(|trip_idx| &self.trips[*trip_idx as usize])
            .collect()
    }

    pub fn stop_times_by_route_id(&self, route_id: &str) -> Option<Vec<&[StopTime]>> {
        let route_idx = self.route_lookup.get(route_id)?;
        Some(self.stop_times_by_route_idx(*route_idx))
    }

    pub fn stop_times_by_route_idx(&self, route_idx: u32) -> Vec<&[StopTime]> {
        self.route_to_trips[route_idx as usize]
            .iter()
            .map(|trip_idx| self.stop_times_by_trip_idx(*trip_idx))
            .collect()
    }

    pub fn routes_by_stop_id(&self, stop_id: &str) -> Option<Vec<&Route>> {
        self.trips_by_stop_id(stop_id)?
            .into_par_iter()
            .map(|trip| self.route_by_trip_id(&trip.id))
            .collect()
    }

    pub fn raptors_by_route_id(&self, route_id: &str) -> Option<Vec<&RaptorRoute>> {
        let route_idx = self.route_lookup.get(route_id)?;
        Some(self.raptors_by_route_idx(*route_idx))
    }

    pub fn raptors_by_route_idx(&self, route_idx: u32) -> Vec<&RaptorRoute> {
        self.route_to_raptors[route_idx as usize]
            .iter()
            .map(|raptor_idx| &self.raptor_routes[*raptor_idx as usize])
            .collect()
    }

    pub fn raptors_by_stop_id(&self, stop_id: &str) -> Option<Vec<&RaptorRoute>> {
        let stop_idx = self.stop_lookup.get(stop_id)?;
        Some(self.raptors_by_stop_idx(*stop_idx))
    }

    pub fn raptors_by_stop_idx(&self, stop_idx: u32) -> Vec<&RaptorRoute> {
        self.stop_to_raptors[stop_idx as usize]
            .iter()
            .map(|raptor_idx| &self.raptor_routes[*raptor_idx as usize])
            .collect()
    }

    /// Returns all the stop times for a given trip.
    /// If no trip was found with the given id None is returned.
    pub fn stop_times_by_trip_id(&self, trip_id: &str) -> Option<&[StopTime]> {
        let trip_idx = self.trip_lookup.get(trip_id)?;
        Some(self.stop_times_by_trip_idx(*trip_idx))
    }

    pub fn stop_times_by_trip_idx(&self, trip_idx: u32) -> &[StopTime] {
        let slice = self.trip_to_stop_slice[trip_idx as usize];
        let start = slice.start_idx as usize;
        let end = start + slice.count as usize;
        &self.stop_times[start..end]
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
                        if let Some(stop_idxs) = self.stop_distance_lookup.get(&cell) {
                            stop_idxs
                                .iter()
                                .filter_map(|stop_idx| {
                                    let stop = &self.stops[*stop_idx as usize];
                                    if stop.coordinate.network_distance(coordinate) <= distance {
                                        Some(stop)
                                    } else {
                                        None
                                    }
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
        stops
            .into_par_iter()
            .map(|stop| self.area_by_stop_idx(stop.index))
            .collect()
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
