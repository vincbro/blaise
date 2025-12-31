use std::{collections::HashMap, sync::Arc};

mod models;
pub use models::*;
use rayon::prelude::*;

use crate::{
    gtfs,
    router::{Router, graph::Location},
    shared::{
        self,
        geo::{AVERAGE_STOP_DISTANCE, Coordinate, Distance},
        time::Duration,
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
        print!("Loading routes...");
        let mut route_lookup: IdToIndex = HashMap::new();
        let mut routes: Vec<Route> = Vec::new();
        gtfs.stream_routes(|(i, route)| {
            let mut value: Route = route.into();
            value.index = i as u32;
            route_lookup.insert(value.id.clone(), i);
            routes.push(value);
        })?;
        self.routes = routes.into();
        self.route_lookup = route_lookup.into();
        println!("OK");

        // Build trip data set
        print!("Loading trips...");
        let mut trip_lookup: IdToIndex = HashMap::new();
        let mut route_to_trips: HashMap<Arc<str>, Vec<Arc<str>>> = HashMap::new();
        let mut trip_to_route: IdToId = HashMap::new();
        let mut trips: Vec<Trip> = Vec::new();
        gtfs.stream_trips(|(i, trip)| {
            let route_index = self.route_lookup.get(trip.route_id.as_str()).unwrap();
            let route_id = self.routes[*route_index].id.clone();
            let value = Trip {
                index: i as u32,
                id: trip.trip_id.into(),
                route_id: route_id.clone(),
                route_index: *route_index as u32,
                raptor_route_index: 0,
                headsign: trip.trip_headsign.map(|val| val.into()),
                short_name: trip.trip_short_name.map(|val| val.into()),
            };
            route_to_trips
                .entry(value.route_id.clone())
                .or_default()
                .push(value.id.clone());
            trip_to_route.insert(value.id.clone(), route_id);
            trip_lookup.insert(value.id.clone(), i);
            trips.push(value);
        })?;
        self.trips = trips.into();
        self.trip_lookup = trip_lookup.into();
        self.trip_to_route = trip_to_route.into();
        let route_to_trips: IdToIds = route_to_trips
            .into_iter()
            .map(|(key, value)| (key, value.into()))
            .collect();
        self.route_to_trips = route_to_trips.into();
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
        let mut last_trip: Option<&Trip> = None;
        let mut start_idx = 0;
        let mut buffer: Vec<StopTime> = vec![];
        gtfs.stream_stop_times(|(i, stop_time)| {
            // TEMP
            let trip_index = self.trip_lookup.get(stop_time.trip_id.as_str()).unwrap();
            let trip = &self.trips[*trip_index];

            if last_trip.is_none() {
                last_trip = Some(trip);
            }

            if let Some(ct) = last_trip
                && ct.index != trip.index
            {
                buffer.par_sort_by_key(|val| val.sequence);
                buffer.iter_mut().enumerate().for_each(|(j, st)| {
                    st.internal_idx = j as u32;
                    st.index = st.start_idx + st.internal_idx;
                });
                let buffer_idxs = buffer.iter().map(|st| st.index as usize).collect();
                trip_to_stop_times.insert(ct.id.clone(), buffer_idxs);
                stop_times.append(&mut buffer);
                last_trip = Some(trip);
                start_idx = i;
            }

            // TEMP
            let stop_index = self.stop_lookup.get(stop_time.stop_id.as_str()).unwrap();
            let stop = &self.stops[*stop_index];

            let mut value: StopTime = stop_time.into();
            value.trip_id = trip.id.clone();
            value.trip_idx = *trip_index as u32;
            value.stop_id = stop.id.clone();
            value.stop_idx = *stop_index as u32;
            value.start_idx = start_idx as u32;
            buffer.push(value);

            stop_to_trips
                .entry(stop.id.clone())
                .or_default()
                .push(trip.id.clone());
        })?;
        buffer.par_sort_by_key(|val| val.sequence);
        buffer.iter_mut().enumerate().for_each(|(j, st)| {
            st.internal_idx = j as u32;
            st.index = st.start_idx + st.internal_idx;
        });
        stop_times.append(&mut buffer);

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

        // Raptor mappings
        // Raptor requires each route's trips to have an identical set of stops.
        // Gtfs does not have this requirement, so we split each route
        // into sub routes that matches these requirements.
        // To build this we are going to go through each route and grab each trip in that route
        // then we are going through each trip and grouping them by there stops
        // with the split trips we can create the raptor route.
        print!("Building raptor bindings...");
        let mut raptor_routes: Vec<RaptorRoute> = Vec::new();
        let mut route_to_raptors: HashMap<Arc<str>, Vec<usize>> = HashMap::new();
        let mut stop_to_raptors: HashMap<Arc<str>, Vec<usize>> = HashMap::new();
        self.routes.iter().for_each(|route| {
            // TEMP but should not fail
            let trips = self.stop_times_by_route_id(&route.id).unwrap();
            let mut raptor_trips: HashMap<Vec<u32>, Vec<u32>> = HashMap::new();
            trips.into_iter().for_each(|trip| {
                let index = trip.first().unwrap().trip_idx;
                let signature: Vec<_> = trip.into_iter().map(|st| st.stop_idx).collect();
                raptor_trips.entry(signature).or_default().push(index);
            });

            raptor_trips.into_iter().for_each(|(key, value)| {
                let index = raptor_routes.len();
                key.iter()
                    .map(|stop_idx| &self.stops[*stop_idx as usize])
                    .for_each(|stop| {
                        stop_to_raptors
                            .entry(stop.id.clone())
                            .or_default()
                            .push(index);
                    });
                route_to_raptors
                    .entry(route.id.clone())
                    .or_default()
                    .push(index);

                let raptor = RaptorRoute {
                    index: index as u32,
                    route_index: route.index,
                    route_id: route.id.clone(),
                    stops: key.into(),
                    trips: value.into(),
                };
                raptor_routes.push(raptor);
            });
        });
        self.raptor_routes = raptor_routes.into();
        let route_to_raptors: IdToIndexes = route_to_raptors
            .into_iter()
            .map(|(id, indexes)| (id, indexes.into()))
            .collect();
        self.route_to_raptors = route_to_raptors.into();

        let stop_to_raptors: IdToIndexes = stop_to_raptors
            .into_iter()
            .map(|(id, indexes)| (id, indexes.into()))
            .collect();
        self.stop_to_raptors = stop_to_raptors.into();
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

    pub fn router(&'_ self, from: Location, to: Location) -> Router<'_> {
        Router::new(self, from, to)
    }
}
