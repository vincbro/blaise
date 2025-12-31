pub mod graph;
use rayon::prelude::*;
use thiserror::Error;

use crate::{
    repository::{RaptorRoute, Repository, Route, Stop, Transfer, Trip},
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

#[derive(Debug, Clone, Default)]
pub struct Parent {
    pub from_stop_idx: u32,
    pub trip_idx: Option<u32>, // None if we walked
    pub time: Time,
}

struct ServingRoute {
    route_idx: u32,
    idx_in_route: u32,
}
pub struct Router<'a> {
    repository: &'a Repository,
    from: Location,
    to: Location,
    departure: Time,
}

impl<'a> Router<'a> {
    pub fn new(repository: &'a Repository, from: Location, to: Location) -> Self {
        Self {
            repository,
            from,
            to,
            departure: Time::now(),
        }
    }

    pub fn departure_at(mut self, departure: Time) -> Self {
        self.departure = departure;
        self
    }

    pub fn solve(self) -> Result<Vec<Parent>, self::Error> {
        // The routes we will explore this round
        let mut active = vec![None; self.repository.raptor_routes.len()];
        // The overall best label we have found for a stop
        let mut tau_star: Vec<Time> = vec![u32::MAX.into(); self.repository.stops.len()];
        // All the stops that changed / improved from the last round
        let mut marked: Vec<bool> = vec![false; self.repository.stops.len()];
        // The per round best label we have found
        let mut labels: Vec<Vec<Option<Time>>> = vec![];
        labels.push(vec![None; self.repository.stops.len()]);
        // Allows us to backtrack and get the full path
        let mut parents: Vec<Vec<Option<Parent>>> = vec![];
        parents.push(vec![None; self.repository.stops.len()]);
        // Run first round
        let from = self.coordinate(&self.from)?;
        let times: Vec<_> = self
            .repository
            .stops_by_coordinate(&from, 500.0.into())
            .into_par_iter()
            .filter(|stop| self.repository.trips_by_stop_id(&stop.id).is_some())
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

            parents[0][index as usize] = Some(Parent {
                from_stop_idx: u32::MAX, // Sentinel for "Start Coordinate"
                trip_idx: None,
                time,
            });
        });

        // Targets
        let to_coord = self.coordinate(&self.to)?;
        let target_stops: Vec<(usize, Duration)> = self
            .repository
            .stops_by_coordinate(&to_coord, 500.0.into()) // same radius as start
            .into_iter()
            .map(|stop| {
                let walk = time_to_walk(stop.coordinate.network_distance(&to_coord));
                (stop.index as usize, walk)
            })
            .collect();
        let mut target_best = self.departure + time_to_walk(from.network_distance(&to_coord));
        let mut target_best_stop: Option<usize> = None;

        // We always start at round 1 since the base round is 0
        let mut round = 1;
        loop {
            // Add a new round to the list
            labels.push(vec![None; self.repository.stops.len()]);
            parents.push(vec![None; self.repository.stops.len()]);
            // FIX: We need a almost full rewrite, what we want to do is find

            // Collect all the routes we should explore based on the stops that got marked
            let marked_stops: Vec<_> = marked
                .par_iter()
                .enumerate()
                .filter_map(|(i, &m)| m.then_some(i))
                .collect();
            println!("Got {} marked stops", marked_stops.len());
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
                    let route = &self.repository.raptor_routes[route_idx];
                    let mut active_trip: Option<&Trip> = None;
                    let mut boarding_stop: u32 = u32::MAX;
                    for (i, stop_idx) in route.stops.iter().enumerate().skip(p_idx as usize) {
                        // PART A
                        if let Some(trip) = active_trip
                            && let Some(arrival_time) = self.get_arrival_time(&trip.id, i)
                        {
                            // FIX Add target check here as well
                            if arrival_time < tau_star[*stop_idx as usize]
                            // && arrival_time < target_best
                            {
                                labels[round][*stop_idx as usize] = Some(arrival_time);
                                tau_star[*stop_idx as usize] = arrival_time;
                                marked[*stop_idx as usize] = true;

                                parents[round][*stop_idx as usize] = Some(Parent {
                                    from_stop_idx: boarding_stop,
                                    trip_idx: Some(trip.index),
                                    time: arrival_time,
                                });
                            }
                        }

                        // PART B
                        let prev_round_arrival =
                            labels[round - 1][*stop_idx as usize].unwrap_or(u32::MAX.into());
                        let current_trip_dep = active_trip
                            .map(|t| self.get_departure_time(&t.id, i).unwrap_or(u32::MAX.into()))
                            .unwrap_or(u32::MAX.into());

                        if prev_round_arrival <= current_trip_dep
                            && let Some(earlier_trip) =
                                self.find_earliest_trip(route, i, prev_round_arrival)
                        {
                            active_trip = Some(earlier_trip);
                            boarding_stop = *stop_idx
                        }
                    }
                });
            // TODO Add walking routes
            let marked_stops: Vec<_> = marked
                .par_iter()
                .enumerate()
                .filter_map(|(i, &m)| m.then_some(i))
                .collect();

            for stop_idx in marked_stops {
                if let Some(transfers) = self.get_transfers(stop_idx) {
                    for transfer in transfers {
                        let walking_arrival =
                            labels[round][stop_idx].unwrap() + self.transfer_duration(transfer);
                        if walking_arrival < tau_star[transfer.to_stop_idx as usize] {
                            labels[round][transfer.to_stop_idx as usize] = Some(walking_arrival);
                            tau_star[transfer.to_stop_idx as usize] = walking_arrival;
                            parents[round][transfer.to_stop_idx as usize] = Some(Parent {
                                from_stop_idx: stop_idx as u32,
                                trip_idx: None,
                                time: walking_arrival,
                            });
                            marked[transfer.to_stop_idx as usize] = true;
                        }
                    }
                }
            }

            for (stop_idx, walk_duration) in target_stops.iter() {
                let arrival_at_goal = tau_star[*stop_idx] + *walk_duration;
                if arrival_at_goal < target_best {
                    target_best = arrival_at_goal;
                    target_best_stop = Some(*stop_idx);
                }
            }
            round += 1;
        }
        println!("DONE WITH {round} ROUNDS");

        if let Some(target_stop) = target_best_stop {
            Ok(self.backtrack(parents, target_stop))
        } else {
            Err(self::Error::FailedToBuildRoute)
        }
    }

    pub fn backtrack(&self, parents: Vec<Vec<Option<Parent>>>, target_stop: usize) -> Vec<Parent> {
        let mut path: Vec<Parent> = Vec::new();
        let mut current_stop = target_stop as u32;

        // Start from the last round and move backwards
        for round in (1..parents.len()).rev() {
            if let Some(parent) = &parents[round][current_stop as usize] {
                path.push(parent.clone());
                current_stop = parent.from_stop_idx;

                // If we reached the start of the first round's walking distance, we are done
                if round == 1 && parents[0][current_stop as usize].is_none() {
                    break;
                }
            }
        }
        path.reverse();
        path
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
            .raptors_by_stop_id(&stop.id)
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

    fn index_in_route(&self, route: &RaptorRoute, stop: &Stop) -> Option<u32> {
        for (index, stop_idx) in route.stops.iter().enumerate() {
            if *stop_idx == stop.index {
                return Some(index as u32);
            }
        }
        None
    }

    fn get_arrival_time(&self, trip_id: &str, index: usize) -> Option<Time> {
        let stop_times = self.repository.stop_times_by_trip_id(trip_id)?;
        if index >= stop_times.len() {
            println!("Tried to get stop {index} in trip {trip_id}");
            return None;
        }
        Some(stop_times[index].arrival_time)
    }
    fn get_departure_time(&self, trip_id: &str, index: usize) -> Option<Time> {
        let stop_times = self.repository.stop_times_by_trip_id(trip_id)?;
        if index >= stop_times.len() {
            println!("Tried to get stop {index} in trip {trip_id}");
            return None;
        }
        Some(stop_times[index].departure_time)
    }

    fn get_transfers(&self, index: usize) -> Option<Vec<&Transfer>> {
        let stop = &self.repository.stops[index];
        self.repository.transfers_by_stop_id(&stop.id)
    }

    /// Finds the earliest trip that we can take from current stop based on the time
    fn find_earliest_trip(&self, route: &RaptorRoute, index: usize, time: Time) -> Option<&Trip> {
        let trips: Vec<_> = route
            .trips
            .iter()
            .map(|trip_idx| &self.repository.trips[*trip_idx as usize])
            .filter_map(|trip| self.repository.stop_times_by_trip_id(&trip.id))
            .collect();
        let mut earliest: Option<(u32, Time)> = None;
        for stop_times in trips.into_iter() {
            let stop_time = stop_times[index];
            let departure_time = stop_time.departure_time;
            // Make sure we don't try to catch a trip that has already left
            if departure_time < time {
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

    fn transfer_duration(&self, transfer: &Transfer) -> Duration {
        let from = &self.repository.stops[transfer.from_stop_idx as usize];
        let to = &self.repository.stops[transfer.to_stop_idx as usize];
        let walk_duration = time_to_walk(from.coordinate.network_distance(&to.coordinate));
        if let Some(duration) = transfer.min_transfer_time {
            duration + walk_duration
        } else {
            walk_duration
        }
    }
}

const fn time_to_walk(distance: Distance) -> Duration {
    let duration = (distance.as_meters() / 1.5).ceil() as u32;
    Duration::from_seconds(duration)
}
