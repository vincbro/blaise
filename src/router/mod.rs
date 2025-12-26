pub mod graph;
use rayon::prelude::*;
use thiserror::Error;

use crate::{
    repository::{Repository, Route, Stop, StopTime},
    router::graph::Location,
    shared::{
        geo::{Coordinate, Distance},
        time::{Duration, Time},
    },
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Area id does not match any entry")]
    InvalidAreaID,
    #[error("Stop id does not match any entry")]
    InvalidStopID,
    #[error("A route was found but failed to build it")]
    FailedToBuildRoute,
    #[error("Could not find a route")]
    NoRouteFound,
}

pub struct Leg(u32, u32);

struct ServingRoute {
    route_idx: u32,
    idx_in_route: u32,
}
pub struct Router<'a> {
    repository: &'a Repository,
    from: Location,
    to: Location,
    departure: Time,
    // The per round best label we have found
    labels: Vec<Vec<Option<Time>>>,
    // The overall best label we have found for a stop
    tau_star: Vec<Time>,
    // All the stops that changed / improved from the last round
    marked: Vec<bool>,
    // The active route will hold were to start exploring that route
    active: Vec<Option<u32>>,
    parents: Vec<Vec<Leg>>,
}

impl<'a> Router<'a> {
    pub fn new(repository: &'a Repository, from: Location, to: Location) -> Self {
        Self {
            repository,
            from,
            to,
            departure: Time::now(),
            labels: vec![],
            tau_star: vec![u32::MAX.into(); repository.stops.len()],
            marked: vec![false; repository.stops.len()],
            active: vec![None; repository.routes.len()],
            parents: vec![],
        }
    }

    pub fn departure_at(mut self, departure: Time) -> Self {
        self.departure = departure;
        self
    }

    pub fn solve(mut self) -> Result<(), self::Error> {
        // Run first round
        self.labels.push(vec![None; self.repository.stops.len()]);
        let from = self.coordinate(&self.from)?;
        let times: Vec<_> = self
            .repository
            .stops_by_coordinate(&from, 500.0.into())
            .into_par_iter()
            .filter(|stop| self.repository.trip_by_id(&stop.id).is_none())
            .map(|stop| {
                let duration = time_to_walk(stop.coordinate.network_distance(&from));
                (stop.index, self.departure + duration)
            })
            .collect();
        println!("Found {} valid stops", times.len());
        times.into_iter().for_each(|(index, time)| {
            self.labels[0][index as usize] = Some(time);
            self.tau_star[index as usize] = time;
            self.marked[index as usize] = true;
        });

        // We always start at round 1 since the base round is 0
        let mut round = 1;
        loop {
            // Add a new round to the list
            self.labels.push(vec![None; self.repository.stops.len()]);
            // FIX: We need a almost full rewrite, what we want to do is find

            // Collect all the routes we should explore based on the stops that got marked
            let marked_stops: Vec<_> = self
                .marked
                .par_iter()
                .enumerate()
                .filter_map(|(i, &m)| m.then_some(i))
                .collect();
            if marked_stops.is_empty() {
                break;
            }
            self.marked.fill(false);

            // Pre process
            self.active.fill(None);
            marked_stops.into_iter().for_each(|stop_idx| {
                let services = self.routes_serving_stop(stop_idx);
                for service in services.into_iter() {
                    let r_idx = service.route_idx as usize;
                    let p_idx = service.idx_in_route;
                    if p_idx < self.active[r_idx].unwrap_or(u32::MAX) {
                        self.active[r_idx] = Some(p_idx);
                    }
                }
            });
            self.active
                .par_iter()
                .enumerate()
                .filter_map(|(r_idx, p_idx)| p_idx.map(|p_idx| (r_idx, p_idx)))
                .for_each(|(route_idx, p_idx)| {
                    let route = &self.repository.routes[route_idx];
                    // TEMP
                    let stop_times = self.repository.stop_times_by_route_id(&route.id).unwrap();
                    let mut possible_starts: Vec<_> = stop_times
                        .par_iter()
                        .filter(|st| st.internal_idx == p_idx)
                        .collect();
                    possible_starts.par_sort_by_key(|st| st.departure_time);
                    let mut start: Option<&StopTime> = None;
                    for possible_start in possible_starts.into_iter() {
                        let prev_arrival = self.labels[round - 1][possible_start.stop_idx as usize]
                            .unwrap_or(0.into());
                        if possible_start.departure_time >= prev_arrival {
                            start = Some(possible_start);
                            break;
                        }
                    }

                    if let Some(start) = start {
                        let trip = &self.repository.trips[start.trip_idx as usize];
                        // TEMP
                        let stop_times = self.repository.stop_times_by_trip_id(&trip.id).unwrap();
                    }
                });
        }
        println!("DONE");

        Ok(())
    }

    fn coordinate(&self, location: &Location) -> Result<Coordinate, self::Error> {
        match location {
            Location::Area(id) => self
                .repository
                .coordinate_by_area_id(id)
                .ok_or(self::Error::InvalidAreaID),
            Location::Stop(id) => self
                .repository
                .stop_by_id(id)
                .map(|stop| stop.coordinate)
                .ok_or(self::Error::InvalidStopID),
            Location::Coordinate(coordinate) => Ok(*coordinate),
        }
    }

    fn routes_serving_stop(&self, stop_idx: usize) -> Vec<ServingRoute> {
        let stop = &self.repository.stops[stop_idx];
        self.repository
            .routes_by_stop_id(&stop.id)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|route| {
                self.index_in_route(route, stop)
                    .map(|idx_in_route| ServingRoute {
                        route_idx: route.index,
                        idx_in_route,
                    })
            })
            .collect()
    }

    fn index_in_route(&self, route: &Route, stop: &Stop) -> Option<u32> {
        let trips = self.repository.trips_by_route_id(&route.id)?;
        if trips.is_empty() {
            return None;
        }

        let stop_times = self.repository.stop_times_by_trip_id(&trips[0].id)?;
        for stop_time in stop_times.into_iter() {
            if stop_time.stop_idx == stop.index {
                return Some(stop_time.internal_idx);
            }
        }
        None
    }
}

const fn time_to_walk(distance: Distance) -> Duration {
    let duration = (distance.as_meters() / 1.5).ceil() as u32;
    Duration::from_seconds(duration)
}
