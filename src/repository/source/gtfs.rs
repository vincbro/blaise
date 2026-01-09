use crate::{
    gtfs::{self, Gtfs},
    repository::{
        Area, Cell, RaptorRoute, Repository, Route, Stop, StopTime, StopTimeSlice, Transfer, Trip,
    },
    shared::time::Duration,
};
use rayon::prelude::*;
use std::{collections::HashMap, sync::Arc, time::Instant};
use tracing::debug;

impl Repository {
    pub fn load_gtfs(mut self, mut gtfs: Gtfs) -> Result<Self, gtfs::Error> {
        self.load_stops(&mut gtfs)?;
        self.load_areas(&mut gtfs)?;
        self.load_area_to_stops(&mut gtfs)?;
        self.load_routes(&mut gtfs)?;
        self.load_trips(&mut gtfs)?;
        self.load_transfers(&mut gtfs)?;
        self.load_stop_times(&mut gtfs)?;
        self.generate_geo_hash();
        self.generate_raptor_routes();
        Ok(self)
    }

    fn load_stops(&mut self, gtfs: &mut Gtfs) -> Result<(), gtfs::Error> {
        debug!("Loading stops...");
        let now = Instant::now();
        let mut stop_lookup: HashMap<Arc<str>, u32> = HashMap::new();
        let mut stops: Vec<Stop> = Vec::new();
        gtfs.stream_stops(|(i, stop)| {
            let mut value: Stop = stop.into();
            value.index = i as u32;
            stop_lookup.insert(value.id.clone(), i as u32);
            stops.push(value);
        })?;
        self.stops = stops.into();
        self.stop_lookup = stop_lookup;
        debug!("Loading stops took {:?}", now.elapsed());
        Ok(())
    }

    fn load_areas(&mut self, gtfs: &mut Gtfs) -> Result<(), gtfs::Error> {
        debug!("Loading areas...");
        let now = Instant::now();
        let mut area_lookup: HashMap<Arc<str>, u32> = HashMap::new();
        let mut areas: Vec<Area> = Vec::new();
        gtfs.stream_areas(|(i, area)| {
            let mut value: Area = area.into();
            value.index = i as u32;
            area_lookup.insert(value.id.clone(), i as u32);
            areas.push(value);
        })?;
        self.areas = areas.into();
        self.area_lookup = area_lookup;
        debug!("Loading areas took {:?}", now.elapsed());
        Ok(())
    }

    fn load_area_to_stops(&mut self, gtfs: &mut Gtfs) -> Result<(), gtfs::Error> {
        debug!("Loading area to stops...");
        let now = Instant::now();

        let mut area_to_stops: Vec<Vec<u32>> = vec![Vec::new(); self.areas.len()];
        let mut stop_to_area: Vec<u32> = vec![u32::MAX; self.stops.len()];
        gtfs.stream_stop_areas(|(_, value)| {
            // TEMP
            let stop_idx = self.stop_lookup.get(value.stop_id.as_str()).unwrap();
            // TEMP
            let area_idx = self.area_lookup.get(value.area_id.as_str()).unwrap();

            stop_to_area[*stop_idx as usize] = *area_idx;
            area_to_stops[*area_idx as usize].push(*stop_idx);
        })?;
        self.stop_to_area = stop_to_area.into();
        let area_to_stops: Box<[Box<[u32]>]> =
            area_to_stops.into_iter().map(|val| val.into()).collect();
        self.area_to_stops = area_to_stops;
        debug!("Loading area to stops took {:?}", now.elapsed());
        Ok(())
    }

    fn load_routes(&mut self, gtfs: &mut Gtfs) -> Result<(), gtfs::Error> {
        debug!("Loading routes...");
        let now = Instant::now();
        let mut route_lookup: HashMap<Arc<str>, u32> = HashMap::new();
        let mut routes: Vec<Route> = Vec::new();
        gtfs.stream_routes(|(i, route)| {
            let mut value: Route = route.into();
            value.index = i as u32;
            route_lookup.insert(value.id.clone(), i as u32);
            routes.push(value);
        })?;
        self.routes = routes.into();
        self.route_lookup = route_lookup;
        debug!("Loading routes took {:?}", now.elapsed());
        Ok(())
    }

    fn load_trips(&mut self, gtfs: &mut Gtfs) -> Result<(), gtfs::Error> {
        debug!("Loading trips...");
        let now = Instant::now();
        let mut trip_lookup: HashMap<Arc<str>, u32> = HashMap::new();
        let mut route_to_trips: Vec<Vec<u32>> = vec![Vec::new(); self.routes.len()];
        let mut trip_to_route: Vec<u32> = Vec::new();
        let mut trips: Vec<Trip> = Vec::new();
        gtfs.stream_trips(|(i, trip)| {
            let route_index = self.route_lookup.get(trip.route_id.as_str()).unwrap();
            let route_id = self.routes[*route_index as usize].id.clone();
            let value = Trip {
                index: i as u32,
                id: trip.trip_id.into(),
                route_id: route_id.clone(),
                route_index: *route_index,
                raptor_route_index: 0,
                headsign: trip.trip_headsign.map(|val| val.into()),
                short_name: trip.trip_short_name.map(|val| val.into()),
            };
            route_to_trips[*route_index as usize].push(i as u32);
            trip_to_route.push(*route_index);
            trip_lookup.insert(value.id.clone(), i as u32);
            trips.push(value);
        })?;
        self.trips = trips.into();
        self.trip_lookup = trip_lookup;
        self.trip_to_route = trip_to_route.into();
        let route_to_trips: Box<[Box<[u32]>]> =
            route_to_trips.into_iter().map(|val| val.into()).collect();
        self.route_to_trips = route_to_trips;
        debug!("Loading trips took {:?}", now.elapsed());
        Ok(())
    }

    fn load_transfers(&mut self, gtfs: &mut Gtfs) -> Result<(), gtfs::Error> {
        debug!("Loading transfers...");
        let now = Instant::now();
        let mut transfers: Vec<Transfer> = Vec::new();
        let mut stop_to_transfers: Vec<Vec<u32>> = vec![Vec::new(); self.stops.len()];
        gtfs.stream_transfers(|(i, transfer)| {
            let from_stop_idx = *self
                .stop_lookup
                .get(transfer.from_stop_id.as_str())
                .unwrap();
            let from_stop = &self.stops[from_stop_idx as usize];

            let to_stop_idx = *self.stop_lookup.get(transfer.to_stop_id.as_str()).unwrap();
            let to_stop = &self.stops[to_stop_idx as usize];

            let (from_trip_id, from_trip_idx) = if let Some(trip_id) = transfer.from_trip_id {
                let trip_idx = *self.trip_lookup.get(trip_id.as_str()).unwrap();
                let trip_id = self.trips[trip_idx as usize].id.clone();
                (Some(trip_id), Some(trip_idx))
            } else {
                (None, None)
            };

            let (to_trip_id, to_trip_idx) = if let Some(trip_id) = transfer.to_trip_id {
                let trip_idx = *self.trip_lookup.get(trip_id.as_str()).unwrap();
                let trip_id = self.trips[trip_idx as usize].id.clone();
                (Some(trip_id), Some(trip_idx))
            } else {
                (None, None)
            };

            stop_to_transfers[from_stop.index as usize].push(i as u32);

            let value = Transfer {
                from_stop_id: from_stop.id.clone(),
                from_stop_idx,
                to_stop_id: to_stop.id.clone(),
                to_stop_idx,
                from_trip_id,
                from_trip_idx,
                to_trip_id,
                to_trip_idx,
                min_transfer_time: transfer.min_transfer_time.map(Duration::from_seconds),
            };

            transfers.push(value);
        })?;
        self.transfers = transfers.into();
        let stop_to_transfers: Box<[Box<[u32]>]> = stop_to_transfers
            .into_iter()
            .map(|val| val.into())
            .collect();
        self.stop_to_transfers = stop_to_transfers;
        debug!("Loading transfers took {:?}", now.elapsed());
        Ok(())
    }

    fn load_stop_times(&mut self, gtfs: &mut Gtfs) -> Result<(), gtfs::Error> {
        debug!("Loading stop times...");
        let now = Instant::now();
        let mut trip_to_stop_slice: Vec<StopTimeSlice> = vec![Default::default(); self.trips.len()];
        let mut stop_to_trips: Vec<Vec<u32>> = vec![Vec::new(); self.stops.len()];
        let mut stop_times: Vec<StopTime> = Vec::new();
        let mut last_trip: Option<&Trip> = None;
        let mut start_idx = 0;
        let mut buffer: Vec<StopTime> = vec![];
        gtfs.stream_stop_times(|(i, stop_time)| {
            // TEMP
            let trip_idx = self.trip_lookup.get(stop_time.trip_id.as_str()).unwrap();
            let trip = &self.trips[*trip_idx as usize];

            if last_trip.is_none() {
                last_trip = Some(trip);
            }

            if let Some(ct) = last_trip
                && ct.index != trip.index
            {
                let stop_time_slice = StopTimeSlice {
                    start_idx: start_idx as u32,
                    count: buffer.len() as u32,
                };

                buffer.par_sort_by_key(|val| val.sequence);
                buffer.iter_mut().enumerate().for_each(|(j, st)| {
                    st.internal_idx = j as u32;
                    st.slice = stop_time_slice;
                    st.index = stop_time_slice.start_idx + st.internal_idx;
                });
                trip_to_stop_slice[ct.index as usize] = stop_time_slice;
                stop_times.append(&mut buffer);
                last_trip = Some(trip);
                start_idx = i;
            }

            // TEMP
            let stop_idx = self.stop_lookup.get(stop_time.stop_id.as_str()).unwrap();
            let stop = &self.stops[*stop_idx as usize];

            let mut value: StopTime = stop_time.into();
            value.trip_id = trip.id.clone();
            value.trip_idx = *trip_idx;
            value.stop_id = stop.id.clone();
            value.stop_idx = *stop_idx;
            buffer.push(value);

            stop_to_trips[*stop_idx as usize].push(*trip_idx);
        })?;

        // If there was a last trip add the buffer to it
        if let Some(trip) = last_trip {
            let stop_time_slice = StopTimeSlice {
                start_idx: start_idx as u32,
                count: buffer.len() as u32,
            };
            buffer.par_sort_by_key(|val| val.sequence);
            buffer.iter_mut().enumerate().for_each(|(j, st)| {
                st.internal_idx = j as u32;
                st.slice = stop_time_slice;
                st.index = st.slice.start_idx + st.internal_idx;
            });
            trip_to_stop_slice[trip.index as usize] = stop_time_slice;
            stop_times.append(&mut buffer);
        }

        self.stop_times = stop_times.into();
        self.trip_to_stop_slice = trip_to_stop_slice.into();

        let stop_to_trips: Box<[Box<[u32]>]> =
            stop_to_trips.into_iter().map(|val| val.into()).collect();
        self.stop_to_trips = stop_to_trips;

        debug!("Loading stop times took {:?}", now.elapsed());
        Ok(())
    }

    fn generate_geo_hash(&mut self) {
        // Link area->stop->real world stop (stops that are linked to any trip)
        // This has to be last because it ties togheter alot
        // To save space and not having a O(n^2) operation trying to map each stop
        // to its nearby stops, we are going to map each stop with trips into a grid
        debug!("Generating geo spatial hash...");
        let now = Instant::now();
        let mut stop_distance_lookup: HashMap<Cell, Vec<u32>> = HashMap::new();
        self.stops.iter().for_each(|stop| {
            let cell = stop.coordinate.to_cell();
            stop_distance_lookup
                .entry(cell)
                .or_default()
                .push(stop.index);
        });
        let stop_distance_lookup: HashMap<Cell, Box<[u32]>> = stop_distance_lookup
            .into_iter()
            .map(|(cell, stops)| (cell, stops.into()))
            .collect();
        self.stop_distance_lookup = stop_distance_lookup;
        debug!("Generating geo spatial hash took {:?}", now.elapsed());
    }

    fn generate_raptor_routes(&mut self) {
        // Raptor mappings
        // Raptor requires each route's trips to have an identical set of stops.
        // Gtfs does not have this requirement, so we split each route
        // into sub routes that matches these requirements.
        // To build this we are going to go through each route and grab each trip in that route
        // then we are going through each trip and grouping them by there stops
        // with the split trips we can create the raptor route.
        debug!("Generating raptor routes...");
        let now = Instant::now();
        let mut raptor_routes: Vec<RaptorRoute> = Vec::new();
        let mut route_to_raptors: Vec<Vec<u32>> = vec![Vec::new(); self.routes.len()];
        let mut stop_to_raptors: Vec<Vec<u32>> = vec![Vec::new(); self.stops.len()];
        self.routes.iter().for_each(|route| {
            let trips = self.stop_times_by_route_idx(route.index);
            let mut raptor_trips: HashMap<Vec<u32>, Vec<u32>> = HashMap::new();
            trips.into_iter().for_each(|trip| {
                let index = trip.first().unwrap().trip_idx;
                let signature: Vec<_> = trip.iter().map(|st| st.stop_idx).collect();
                raptor_trips.entry(signature).or_default().push(index);
            });

            raptor_trips.into_iter().for_each(|(key, value)| {
                let index = raptor_routes.len();
                key.iter().for_each(|stop_idx| {
                    stop_to_raptors[*stop_idx as usize].push(index as u32);
                });
                route_to_raptors[route.index as usize].push(index as u32);

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
        let route_to_raptors: Box<[Box<[u32]>]> =
            route_to_raptors.into_iter().map(|val| val.into()).collect();
        self.route_to_raptors = route_to_raptors;

        let stop_to_raptors: Box<[Box<[u32]>]> =
            stop_to_raptors.into_iter().map(|val| val.into()).collect();
        self.stop_to_raptors = stop_to_raptors;
        debug!("Generating raptor routes took {:?}", now.elapsed());
    }
}
