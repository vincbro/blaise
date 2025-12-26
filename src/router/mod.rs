pub mod graph;
use rayon::prelude::*;
use thiserror::Error;

use crate::{
    repository::{Repository, Route, Stop, Trip},
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
    parents: Vec<Vec<Leg>>,
}

impl<'a> Router<'a> {
    pub fn new(repository: &'a Repository, from: Location, to: Location) -> Self {
        Self {
            repository,
            from,
            to,
            departure: Time::now(),
            parents: vec![],
        }
    }

    pub fn departure_at(mut self, departure: Time) -> Self {
        self.departure = departure;
        self
    }

    pub fn solve(self) -> Result<(), self::Error> {
        // The routes we will explore this round
        let mut active = vec![None; self.repository.routes.len()];
        // The overall best label we have found for a stop
        let mut tau_star: Vec<Time> = vec![u32::MAX.into(); self.repository.stops.len()];
        // All the stops that changed / improved from the last round
        let mut marked: Vec<bool> = vec![false; self.repository.stops.len()];
        // The per round best label we have found
        let mut labels: Vec<Vec<Option<Time>>> = vec![];
        labels.push(vec![None; self.repository.stops.len()]);
        // Run first round
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
            labels[0][index as usize] = Some(time);
            tau_star[index as usize] = time;
            marked[index as usize] = true;
        });

        // We always start at round 1 since the base round is 0
        let mut round = 1;
        loop {
            // Add a new round to the list
            labels.push(vec![None; self.repository.stops.len()]);
            // FIX: We need a almost full rewrite, what we want to do is find

            // Collect all the routes we should explore based on the stops that got marked
            let marked_stops: Vec<_> = marked
                .par_iter()
                .enumerate()
                .filter_map(|(i, &m)| m.then_some(i))
                .collect();
            if marked_stops.is_empty() {
                break;
            }
            marked.fill(false);

            // Pre process
            active.fill(None);
            marked_stops.into_iter().for_each(|stop_idx| {
                let services = self.routes_serving_stop(stop_idx);
                for service in services.into_iter() {
                    let r_idx = service.route_idx as usize;
                    let p_idx = service.idx_in_route;
                    if p_idx < active[r_idx].unwrap_or(u32::MAX) {
                        active[r_idx] = Some(p_idx);
                    }
                }
            });
            active
                .iter()
                .enumerate()
                .filter_map(|(r_idx, p_idx)| p_idx.map(|p_idx| (r_idx, p_idx)))
                .for_each(|(route_idx, p_idx)| {
                    let route = &self.repository.routes[route_idx];
                    let route_stops = self.get_route_stops(&route.id).unwrap();
                    let mut active_trip: Option<&Trip> = None;
                    for (i, stop_idx) in route_stops.into_iter().enumerate().skip(p_idx as usize) {
                        // PART A
                        if let Some(trip) = active_trip {
                            let arrival_time = self.get_arrival_time(&trip.id, i).unwrap();
                            // FIX Add target check here as well
                            if arrival_time < tau_star[stop_idx as usize] {
                                labels[round][stop_idx as usize] = Some(arrival_time);
                                tau_star[stop_idx as usize] = arrival_time;
                                marked[stop_idx as usize] = true;
                            }
                        }

                        // PART B
                        let prev_round_arrival =
                            labels[round - 1][stop_idx as usize].unwrap_or(u32::MAX.into());
                        let current_trip_dep = active_trip
                            .map(|t| self.get_departure_time(&t.id, i).unwrap_or(u32::MAX.into()))
                            .unwrap_or(u32::MAX.into());

                        if prev_round_arrival <= current_trip_dep
                            && let Some(earlier_trip) =
                                self.find_earliest_trip(&route.id, i, prev_round_arrival)
                        {
                            active_trip = Some(earlier_trip);
                        }
                    }
                });
            round += 1;
        }
        println!("DONE WITH {round} ROUNDS");

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

    fn get_route_stops(&self, route_id: &str) -> Option<Vec<u32>> {
        let trips = self.repository.trips_by_route_id(route_id)?;
        let first_trip = trips.first()?;
        let stop_times = self.repository.stop_times_by_trip_id(&first_trip.id)?;
        Some(stop_times.into_iter().map(|st| st.stop_idx).collect())
    }

    fn get_arrival_time(&self, trip_id: &str, index: usize) -> Option<Time> {
        let stop_times = self.repository.stop_times_by_trip_id(trip_id)?;
        Some(stop_times[index].arrival_time)
    }
    fn get_departure_time(&self, trip_id: &str, index: usize) -> Option<Time> {
        let stop_times = self.repository.stop_times_by_trip_id(trip_id)?;
        Some(stop_times[index].departure_time)
    }

    /// Finds the earliest trip that we can take from current stop based on the time
    fn find_earliest_trip(&self, route_id: &str, index: usize, time: Time) -> Option<&Trip> {
        let trips = self.repository.stop_times_by_route_id(route_id)?;
        let mut earliest: Option<(u32, Time)> = None;
        for stop_times in trips.into_iter() {
            let stop_time = stop_times[index];
            let departure_time = stop_time.departure_time;
            // Make sure we don't try to catch a trip that has already left
            if departure_time > time {
                continue;
            }
            if let Some((_, time_to_beat)) = earliest {
                if departure_time < time_to_beat {
                    earliest = Some((stop_time.trip_idx, departure_time));
                }
            } else {
                earliest = Some((stop_time.trip_idx, departure_time));
            }
        }

        if let Some((trip_idx, _)) = earliest {
            Some(&self.repository.trips[trip_idx as usize])
        } else {
            None
        }
    }
}

const fn time_to_walk(distance: Distance) -> Duration {
    let duration = (distance.as_meters() / 1.5).ceil() as u32;
    Duration::from_seconds(duration)
}
