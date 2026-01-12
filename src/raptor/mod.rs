mod allocator;
mod itinerary;
mod location;
mod state;

pub use allocator::*;
pub use itinerary::*;
pub use location::*;
pub use state::*;

use crate::{
    repository::{RaptorRoute, Repository, StopTime, Transfer, Trip},
    shared::{
        geo::{Coordinate, Distance},
        time::{Duration, Time},
    },
};
use rayon::prelude::*;
use thiserror::Error;
use tracing::{debug, warn};

pub const MAX_ROUNDS: usize = 15;

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

struct ServingRoute {
    route_idx: u32,
    idx_in_route: u32,
}

/// The execution engine for the Round-Based Public Transit Routing (RAPTOR) algorithm.
///
/// This struct holds the search parameters and a reference to the underlying transit
/// [`Repository`]. It is designed to be short-lived, typically created via
/// [`Repository::router`].
///
/// # Search Logic
/// RAPTOR explores the network in "rounds." Round `K` finds all stops reachable
/// with exactly `K` trips. This structure ensures that we only explore the
/// necessary graph edges based on the `departure` time and `walk_distance` constraints.
pub struct Raptor<'a> {
    repository: &'a Repository,
    from: Location,
    to: Location,
    departure: Time,
    walk_distance: Distance,
    allocator: Option<Allocator>,
}

impl<'a> Raptor<'a> {
    /// Creates a new RAPTOR search instance for a specific origin and destination.
    ///
    /// By default, the search uses the current system time for departure and
    /// a standard walking distance. These can be customized using the builder
    /// methods before calling [`solve`].
    ///
    /// # Arguments
    /// * `repository` - A reference to the static transit data.
    /// * `from` - The starting location (Stop, Area, or Coordinate).
    /// * `to` - The target destination.
    pub fn new(repository: &'a Repository, from: Location, to: Location) -> Self {
        Self {
            repository,
            from,
            to,
            departure: Time::now(),
            walk_distance: 500.0.into(),
            allocator: None,
        }
    }

    pub fn with_allocator(mut self, allocator: Allocator) -> Self {
        self.allocator = Some(allocator);
        self
    }

    /// Sets the earliest time the journey can begin.
    ///
    /// The algorithm will only consider trips that depart at or after this time.
    /// Note that earlier departure times may result in different optimal paths
    /// even for the same origin/destination.
    pub fn departure_at(mut self, departure: Time) -> Self {
        self.departure = departure;
        self
    }

    /// Wrapper around slove_with_allocator but creates the allocator internally.
    ///
    /// Executes the multi-criteria search and returns the optimal itinerary.
    ///
    /// This is the most computationally expensive part of the process, involving
    /// parallelized route scanning and transfer calculations.
    ///
    /// # Returns
    /// * `Ok(Itinerary)` - The best path found based on arrival time.
    /// * `Err(Error)` - Returns an error if no path exists or if the search
    ///   parameters are invalid.
    ///
    /// # Performance
    /// This method leverages the parallel optimizations in the underlying [`Repository`].
    /// Execution time typically scales with the number of possible routes between
    /// the origin and destination.
    pub fn solve(self) -> Result<Itinerary, self::Error> {
        let mut allocator = Allocator::new(self.repository);
        self.solve_with_allocator(&mut allocator)
    }

    /// Executes the multi-criteria search and returns the optimal itinerary.
    ///
    /// This is the most computationally expensive part of the process, involving
    /// parallelized route scanning and transfer calculations.
    ///
    /// # Returns
    /// * `Ok(Itinerary)` - The best path found based on arrival time.
    /// * `Err(Error)` - Returns an error if no path exists or if the search
    ///   parameters are invalid.
    ///
    /// # Performance
    /// This method leverages the parallel optimizations in the underlying [`Repository`].
    /// Execution time typically scales with the number of possible routes between
    /// the origin and destination.
    pub fn solve_with_allocator(self, allocator: &mut Allocator) -> Result<Itinerary, self::Error> {
        let mut active = vec![None; self.repository.raptor_routes.len()];

        let from_coord = self.coordinate(&self.from)?;
        let updates = self
            .repository
            .stops_by_coordinate(&from_coord, self.walk_distance)
            .into_par_iter()
            .filter(|stop| !self.repository.trips_by_stop_idx(stop.index).is_empty())
            .map(|stop| {
                let walk_duration = time_to_walk(stop.coordinate.network_distance(&from_coord));
                let arrival_time = self.departure + walk_duration;
                Update::new(
                    stop.index,
                    arrival_time,
                    Parent::new_walk(
                        from_coord.into(),
                        stop.index.into(),
                        self.departure,
                        arrival_time,
                    ),
                )
            });
        allocator.updates.par_extend(updates);
        allocator.run_updates(0);

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
        let mut round: usize = 1;
        loop {
            if round >= MAX_ROUNDS {
                warn!("Hit round limit!");
                break;
            }
            allocator.swap_labels();

            let marked_stops = allocator.get_marked_stops();
            debug!("Got {} marked stops", marked_stops.len());
            if marked_stops.is_empty() {
                break;
            }
            allocator.marked_stops.fill(false);

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
                let routes = self.routes_serving_stop(stop_idx as u32);
                for route in routes.into_iter() {
                    let r_idx = route.route_idx as usize;
                    let p_idx = route.idx_in_route;
                    if p_idx < active[r_idx].unwrap_or(u32::MAX) {
                        active[r_idx] = Some(p_idx);
                    }
                }
            });
            let updates = active
                .par_iter()
                .enumerate()
                .filter_map(|(r_idx, p_idx)| p_idx.map(|p_idx| (r_idx, p_idx)))
                .map(|(route_idx, p_idx)| {
                    // We walk down each route starting from
                    // the earliest stop in the route we updated last round

                    let route = &self.repository.raptor_routes[route_idx];
                    let mut active_trip: Option<&Trip> = None;
                    let mut boarding_stop: u32 = u32::MAX;
                    let mut boarding_p: usize = usize::MAX;

                    // --- Buffers ---
                    let mut updates_buffer: Vec<Update> = Vec::with_capacity(64);
                    let mut earliest_trip_buffer: Vec<&[StopTime]> = Vec::with_capacity(64);

                    for (i, stop_idx) in route.stops.iter().enumerate().skip(p_idx as usize) {
                        // PART A
                        // Walk a certain trip and mark any stop were we improve our time
                        if let Some(trip) = active_trip
                            && let arrival_time = self.get_arrival_time(trip.index, i)
                            && arrival_time
                                < allocator.tau_star[*stop_idx as usize].unwrap_or(u32::MAX.into())
                            && arrival_time < target_best
                        {
                            updates_buffer.push(Update::new(
                                *stop_idx,
                                arrival_time,
                                Parent::new_transit(
                                    boarding_stop.into(),
                                    (*stop_idx).into(),
                                    trip.index,
                                    self.get_departure_time(trip.index, boarding_p),
                                    arrival_time,
                                ),
                            ));
                        }

                        // PART B
                        // See if we could have catched an earlier trip to get to were we currently are
                        let prev_round_arrival =
                            allocator.prev_labels[*stop_idx as usize].unwrap_or(u32::MAX.into());
                        let current_trip_dep = active_trip
                            .map(|t| self.get_departure_time(t.index, i))
                            .unwrap_or(u32::MAX.into());

                        if prev_round_arrival <= current_trip_dep
                            && let Some(earlier_trip) = self.find_earliest_trip(
                                route,
                                i,
                                prev_round_arrival,
                                &mut earliest_trip_buffer,
                            )
                        {
                            active_trip = Some(earlier_trip);
                            boarding_stop = *stop_idx;
                            boarding_p = i;
                        }
                    }
                    updates_buffer
                })
                .flatten();
            allocator.updates.par_extend(updates);
            allocator.run_updates(round);

            let updates = allocator
                .get_marked_stops()
                .into_par_iter()
                .map(|stop_idx| {
                    let mut updates: Vec<Update> = Vec::with_capacity(64);
                    // All the possible transfers
                    for transfer in self.repository.transfers_by_stop_idx(stop_idx as u32) {
                        let departure_time =
                            allocator.curr_labels[stop_idx].unwrap_or(u32::MAX.into());
                        let arrival_time = departure_time + self.transfer_duration(transfer);
                        if arrival_time
                            < allocator.tau_star[transfer.to_stop_idx as usize]
                                .unwrap_or(u32::MAX.into())
                            && arrival_time < target_best
                        {
                            updates.push(Update::new(
                                transfer.to_stop_idx,
                                arrival_time,
                                Parent::new_transfer(
                                    (stop_idx as u32).into(),
                                    transfer.to_stop_idx.into(),
                                    departure_time,
                                    arrival_time,
                                ),
                            ));
                        }
                    }

                    let current_stop = &self.repository.stops[stop_idx];
                    self.repository
                        .nearby_stops_by_stop_idx(current_stop.index)
                        .into_iter()
                        .for_each(|next_stop| {
                            let walking_distance = current_stop
                                .coordinate
                                .network_distance(&next_stop.coordinate);
                            let departure_time =
                                allocator.curr_labels[stop_idx].unwrap_or(u32::MAX.into());
                            let arrival_time = departure_time + time_to_walk(walking_distance);
                            if arrival_time
                                < allocator.tau_star[next_stop.index as usize]
                                    .unwrap_or(u32::MAX.into())
                                && arrival_time < target_best
                            {
                                updates.push(Update::new(
                                    next_stop.index,
                                    arrival_time,
                                    Parent::new_walk(
                                        (stop_idx as u32).into(),
                                        next_stop.index.into(),
                                        departure_time,
                                        arrival_time,
                                    ),
                                ));
                            }
                        });
                    updates
                })
                .flatten();
            allocator.updates.par_extend(updates);
            allocator.run_updates(round);

            target_stops
                .iter()
                .filter_map(|(stop_idx, walk_duration)| {
                    let tau_star = allocator.tau_star[*stop_idx];
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

        if let Some(target_stop) = target_best_stop
            && let Some(target_round) = target_best_round
        {
            let path = self.backtrack(&allocator, to_coord, target_stop, target_round)?;
            Ok(Itinerary::new(self.from, self.to, path, self.repository))
        } else {
            Err(self::Error::NoRouteFound)
        }
    }

    fn backtrack(
        &self,
        allocator: &Allocator,
        to_coord: Coordinate,
        target_stop: usize,
        target_round: usize,
    ) -> Result<Vec<Parent>, self::Error> {
        let mut path: Vec<Parent> = Vec::new();

        let mut current_point: Point = (target_stop as u32).into();
        let mut current_round = target_round;

        while let Point::Stop(current_stop) = current_point {
            if let Some(parent) = &allocator.get_parents(current_round)[current_stop as usize] {
                path.push(*parent);
                current_point = parent.from;
                // If we are on a transit we decrese the round else we don't since
                // transfers does not count as a round switch
                if parent.parent_type.is_transit() {
                    current_round -= 1;
                } else if current_round == 0 {
                    break;
                }
            } else {
                return Err(Error::FailedToBuildRoute);
            }
        }
        path.reverse();

        if let Some(last_parent) = path.pop() {
            let final_stop_coord = self.repository.stops[target_stop].coordinate;
            let dist_to_target = final_stop_coord.network_distance(&to_coord);
            let walk_to_target = time_to_walk(dist_to_target);

            if walk_to_target == 0.into() {
                path.push(last_parent);
            } else {
                match last_parent.parent_type.is_walk() {
                    true => {
                        // Merge: From the start of the last walk, straight to the final coord
                        path.push(Parent::new_walk(
                            last_parent.from,
                            to_coord.into(),
                            last_parent.arrival_time,
                            last_parent.arrival_time + walk_to_target,
                        ));
                    }
                    false => {
                        // Keep the transit leg, then add a final walk leg
                        path.push(last_parent);
                        path.push(Parent::new_walk(
                            Point::Stop(target_stop as u32),
                            to_coord.into(),
                            last_parent.arrival_time,
                            last_parent.arrival_time + walk_to_target,
                        ));
                    }
                }
            }
        }
        Ok(path)
    }

    fn coordinate(&self, location: &Location) -> Result<Coordinate, self::Error> {
        match location {
            Location::Area(id) => {
                let area_idx = self
                    .repository
                    .area_by_id(id)
                    .ok_or(self::Error::InvalidAreaID)?;
                Ok(self.repository.coordinate_by_area_idx(area_idx.index))
            }
            Location::Stop(id) => self
                .repository
                .stop_by_id(id)
                .map(|stop| stop.coordinate)
                .ok_or(self::Error::InvalidStopID),
            Location::Coordinate(coordinate) => Ok(*coordinate),
        }
    }

    fn routes_serving_stop(&self, stop_idx: u32) -> Vec<ServingRoute> {
        self.repository
            .raptors_by_stop_idx(stop_idx)
            .into_iter()
            .filter_map(|route| {
                self.index_in_route(route, stop_idx)
                    .map(|idx_in_route| ServingRoute {
                        route_idx: route.index,
                        idx_in_route,
                    })
            })
            .collect()
    }

    fn index_in_route(&self, route: &RaptorRoute, stop_idx: u32) -> Option<u32> {
        for (index, route_stop_idx) in route.stops.iter().enumerate() {
            if *route_stop_idx == stop_idx {
                return Some(index as u32);
            }
        }
        None
    }

    fn get_arrival_time(&self, trip_idx: u32, index: usize) -> Time {
        let stop_times = self.repository.stop_times_by_trip_idx(trip_idx);
        stop_times[index].arrival_time
    }

    fn get_departure_time(&self, trip_idx: u32, index: usize) -> Time {
        let stop_times = self.repository.stop_times_by_trip_idx(trip_idx);
        stop_times[index].departure_time
    }

    /// Finds the earliest trip that we can take from current stop based on the time
    fn find_earliest_trip(
        &self,
        route: &RaptorRoute,
        index: usize,
        time: Time,
        buffer: &mut Vec<&'a [StopTime]>,
    ) -> Option<&Trip> {
        buffer.clear();
        buffer.extend(
            route
                .trips
                .iter()
                .map(|trip_idx| self.repository.stop_times_by_trip_idx(*trip_idx)),
        );
        let mut earliest: Option<(u32, Time)> = None;
        for stop_times in buffer.iter() {
            let stop_time = &stop_times[index];
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
        if let Some(duration) = transfer.min_transfer_time {
            duration
        } else {
            let from = &self.repository.stops[transfer.from_stop_idx as usize];
            let to = &self.repository.stops[transfer.to_stop_idx as usize];
            time_to_walk(from.coordinate.network_distance(&to.coordinate))
        }
    }
}

const fn time_to_walk(distance: Distance) -> Duration {
    let duration = (distance.as_meters() / 1.5).ceil() as u32;
    Duration::from_seconds(duration)
}
