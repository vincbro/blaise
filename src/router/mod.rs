pub mod graph;

use rayon::prelude::*;
use thiserror::Error;

use crate::{
    repository::{RaptorRoute, Repository, Stop, Transfer, Trip},
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
    pub arrival_time: Time,
}

#[derive(Debug, Clone, Default)]
pub struct Update {
    pub stop_idx: u32,
    pub arrival_time: Time,
    pub parent: Parent,
}

struct ServingRoute {
    route_idx: u32,
    idx_in_route: u32,
}

pub struct State {
    tau_star: Vec<Option<Time>>,
    marked: Vec<bool>,
    labels: Vec<Vec<Option<Time>>>,
    parents: Vec<Vec<Option<Parent>>>,
}

impl State {
    pub fn new(repository: &Repository) -> Self {
        Self {
            tau_star: vec![None; repository.stops.len()],
            marked: vec![false; repository.stops.len()],
            labels: vec![],
            parents: vec![],
        }
    }

    pub fn apply_updates(&mut self, round: usize, updates: Vec<Update>) {
        updates.into_iter().for_each(|update| {
            let best_time = self.tau_star[update.stop_idx as usize].unwrap_or(u32::MAX.into());
            if update.arrival_time < best_time {
                self.labels[round][update.stop_idx as usize] = Some(update.arrival_time);
                self.parents[round][update.stop_idx as usize] = Some(update.parent);
                self.tau_star[update.stop_idx as usize] = Some(update.arrival_time);
                self.marked[update.stop_idx as usize] = true;
            }
        })
    }

    fn marked_stops(&self) -> Vec<usize> {
        self.marked
            .par_iter()
            .enumerate()
            .filter_map(|(i, &m)| m.then_some(i))
            .collect()
    }
}

pub struct Router<'a> {
    repository: &'a Repository,
    from: Location,
    to: Location,
    departure: Time,
    walk_distance: Distance,
}

impl<'a> Router<'a> {
    pub fn new(repository: &'a Repository, from: Location, to: Location) -> Self {
        Self {
            repository,
            from,
            to,
            departure: Time::now(),
            walk_distance: 500.0.into(),
        }
    }

    pub fn departure_at(mut self, departure: Time) -> Self {
        self.departure = departure;
        self
    }

    pub fn solve(self) -> Result<Vec<Parent>, self::Error> {
        let mut active = vec![None; self.repository.raptor_routes.len()];
        let mut state = State::new(self.repository);
        state.labels.push(vec![None; self.repository.stops.len()]);
        state.parents.push(vec![None; self.repository.stops.len()]);

        let from_coord = self.coordinate(&self.from)?;
        let updates: Vec<_> = self
            .repository
            .stops_by_coordinate(&from_coord, self.walk_distance)
            .into_par_iter()
            // We filter out all GTFS stops that do serve a trip
            .filter(|stop| self.repository.trips_by_stop_id(&stop.id).is_some())
            .map(|stop| {
                let walk_duration = time_to_walk(stop.coordinate.network_distance(&from_coord));
                let arrival_time = self.departure + walk_duration;
                Update {
                    stop_idx: stop.index,
                    arrival_time,
                    parent: Parent {
                        from_stop_idx: u32::MAX, // Sentinel for "Start Coordinate"
                        trip_idx: None,
                        arrival_time,
                    },
                }
            })
            .collect();
        state.apply_updates(0, updates);

        // Targets
        let to_coord = self.coordinate(&self.to)?;
        let target_stops: Vec<(usize, Duration)> = self
            .repository
            .stops_by_coordinate(&to_coord, self.walk_distance) // same radius as start
            .into_iter()
            .map(|stop| {
                let walk = time_to_walk(stop.coordinate.network_distance(&to_coord));
                (stop.index as usize, walk)
            })
            .collect();

        // Stores the current best path we have found
        let mut target_best: Time = u32::MAX.into();
        let mut target_best_stop: Option<usize> = None;
        let mut target_best_round: Option<usize> = None;

        // We always start at round 1 since the base round is 0
        let mut round = 1;
        loop {
            // Add a new round to the list
            state.labels.push(vec![None; self.repository.stops.len()]);
            state.parents.push(vec![None; self.repository.stops.len()]);

            let marked_stops = state.marked_stops();
            println!("Got {} marked stops", marked_stops.len());
            // If we don't improve we have found th
            if marked_stops.is_empty() {
                break;
            }
            state.marked.fill(false);

            // Pre process
            active.fill(None);
            marked_stops.into_iter().for_each(|stop_idx| {
                // We look at all the routes that serve a stop
                // for each route that serve a route we store the earliest stop in that route
                // that we serve
                // Example: This is a the stops in a route
                // we have marked 1 3 6 as improvments
                // so we want to make sure that we only exlopre this route once and from the earliest stop
                // in this case it will be 1
                // 0 1 2 3 4 5 6 7 8
                //   ^   ^     ^
                let services = self.routes_serving_stop(stop_idx);
                for service in services.into_iter() {
                    let r_idx = service.route_idx as usize;
                    let p_idx = service.idx_in_route;
                    if p_idx < active[r_idx].unwrap_or(u32::MAX) {
                        active[r_idx] = Some(p_idx);
                    }
                }
            });
            let updates: Vec<Update> = active
                .par_iter()
                .enumerate()
                .filter_map(|(r_idx, p_idx)| p_idx.map(|p_idx| (r_idx, p_idx)))
                .map(|(route_idx, p_idx)| {
                    // We walk down each route starting from
                    // the earliest stop in the route we updated last round
                    let mut updates: Vec<Update> = vec![];
                    let route = &self.repository.raptor_routes[route_idx];
                    let mut active_trip: Option<&Trip> = None;
                    let mut boarding_stop: u32 = u32::MAX;
                    for (i, stop_idx) in route.stops.iter().enumerate().skip(p_idx as usize) {
                        // PART A
                        // Walk a certain trip and mark any stop were we improve our time
                        if let Some(trip) = active_trip
                            && let Some(arrival_time) = self.get_arrival_time(&trip.id, i)
                            && arrival_time
                                < state.tau_star[*stop_idx as usize].unwrap_or(u32::MAX.into())
                            && arrival_time < target_best
                        {
                            updates.push(Update {
                                stop_idx: *stop_idx,
                                arrival_time,
                                parent: Parent {
                                    from_stop_idx: boarding_stop,
                                    trip_idx: Some(trip.index),
                                    arrival_time,
                                },
                            });
                        }

                        // PART B
                        // See if we could have catched an earlier trip to get to were we currently are
                        let prev_round_arrival =
                            state.labels[round - 1][*stop_idx as usize].unwrap_or(u32::MAX.into());
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
                    updates
                })
                .flatten()
                .collect();

            // Apply all the updates and store all the updated stops
            state.apply_updates(round, updates);

            let updates: Vec<_> = state
                .marked_stops()
                .into_par_iter()
                .map(|stop_idx| {
                    let mut updates: Vec<Update> = vec![];
                    // All the possible transfers
                    if let Some(transfers) = self.get_transfers(stop_idx) {
                        for transfer in transfers {
                            let arrival_time = state.labels[round][stop_idx].unwrap()
                                + self.transfer_duration(transfer);
                            if arrival_time
                                < state.tau_star[transfer.to_stop_idx as usize]
                                    .unwrap_or(u32::MAX.into())
                            {
                                updates.push(Update {
                                    stop_idx: transfer.to_stop_idx,
                                    arrival_time,
                                    parent: Parent {
                                        from_stop_idx: stop_idx as u32,
                                        trip_idx: None,
                                        arrival_time,
                                    },
                                });
                            }
                        }
                    }
                    // All the possible walks
                    let current_stop = &self.repository.stops[stop_idx];
                    self.repository
                        .stops_by_coordinate(&current_stop.coordinate, self.walk_distance)
                        .into_iter()
                        .filter(|next_stop| next_stop.index != current_stop.index)
                        .for_each(|next_stop| {
                            let walking_distance = current_stop
                                .coordinate
                                .network_distance(&next_stop.coordinate);
                            let arrival_time = state.labels[round][stop_idx].unwrap()
                                + time_to_walk(walking_distance);
                            if arrival_time
                                < state.tau_star[next_stop.index as usize]
                                    .unwrap_or(u32::MAX.into())
                            {
                                updates.push(Update {
                                    stop_idx: next_stop.index,
                                    arrival_time,
                                    parent: Parent {
                                        from_stop_idx: stop_idx as u32,
                                        trip_idx: None,
                                        arrival_time,
                                    },
                                });
                            }
                        });
                    updates
                })
                .flatten()
                .collect();
            state.apply_updates(round, updates);

            target_stops
                .iter()
                .filter_map(|(stop_idx, walk_duration)| {
                    let tau_star = state.tau_star[*stop_idx];
                    tau_star.map(|tau_star| (stop_idx, walk_duration, tau_star))
                })
                .for_each(|(stop_idx, walk_duration, tau_star)| {
                    let arrival_at_goal = tau_star + *walk_duration;
                    if arrival_at_goal < target_best {
                        target_best = arrival_at_goal;
                        target_best_stop = Some(*stop_idx);
                        target_best_round = Some(round);
                    }
                });
            round += 1;
        }
        println!("DONE WITH {round} ROUNDS");

        if let Some(target_stop) = target_best_stop
            && let Some(target_round) = target_best_round
        {
            Ok(self.backtrack(state.parents, target_stop, target_round))
        } else {
            Err(self::Error::FailedToBuildRoute)
        }
    }

    fn backtrack(
        &self,
        parents: Vec<Vec<Option<Parent>>>,
        target_stop: usize,
        target_round: usize,
    ) -> Vec<Parent> {
        let mut path: Vec<Parent> = Vec::new();
        let mut current_stop = target_stop as u32;
        let mut current_round = target_round;

        while current_stop != u32::MAX {
            if let Some(parent) = &parents[current_round][current_stop as usize] {
                path.push(parent.clone());
                let next_stop = parent.from_stop_idx;

                if parent.trip_idx.is_some() {
                    current_round -= 1;
                } else if current_round == 0 {
                    break;
                }

                current_stop = next_stop;
            } else if current_round > 0 {
                current_round -= 1;
            } else {
                break;
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
