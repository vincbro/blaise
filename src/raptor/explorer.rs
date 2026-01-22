use crate::{
    raptor::{
        Allocator, LazyBuffer, Parent, Update, find_earliest_trip, get_arrival_time,
        get_departure_time, time_to_walk, transfer_duration,
    },
    repository::{Repository, Trip},
};
use rayon::prelude::*;

/// Explores all active routes and add any updates to the update buffer in the allocator
pub fn explore_routes(repository: &Repository, allocator: &mut Allocator) {
    let updates = allocator
        .active
        .par_iter()
        .enumerate()
        .map_init(
            || LazyBuffer::new(32),
            |buffer, (route_idx, p_idx)| {
                let p_idx = match p_idx {
                    Some(p_idx) => *p_idx,
                    None => return vec![],
                };
                // We walk down each route starting from
                // the earliest stop in the route we updated last round

                let route = &repository.raptor_routes[route_idx];
                let mut active_trip: Option<&Trip> = None;
                let mut boarding_stop: u32 = u32::MAX;
                let mut boarding_p: usize = usize::MAX;

                for (i, stop_idx) in route.stops.iter().enumerate().skip(p_idx as usize) {
                    // PART A
                    // Walk a certain trip and mark any stop were we improve our time
                    if let Some(trip) = active_trip
                        && let arrival_time = get_arrival_time(repository, trip.index, i)
                        && arrival_time
                            < allocator.tau_star[*stop_idx as usize].unwrap_or(u32::MAX.into())
                        && arrival_time < allocator.target.tau_star
                    {
                        buffer.push(Update::new(
                            *stop_idx,
                            arrival_time,
                            Parent::new_transit(
                                boarding_stop.into(),
                                (*stop_idx).into(),
                                trip.index,
                                get_departure_time(repository, trip.index, boarding_p),
                                arrival_time,
                            ),
                        ));
                    }

                    // PART B
                    // See if we could have catched an earlier trip to get to were we currently are
                    let prev_round_arrival =
                        allocator.prev_labels[*stop_idx as usize].unwrap_or(u32::MAX.into());
                    let current_trip_dep = active_trip
                        .map(|t| get_departure_time(repository, t.index, i))
                        .unwrap_or(u32::MAX.into());

                    if prev_round_arrival <= current_trip_dep
                        && let Some(earlier_trip) =
                            find_earliest_trip(repository, route, i, prev_round_arrival)
                    {
                        active_trip = Some(earlier_trip);
                        boarding_stop = *stop_idx;
                        boarding_p = i;
                    }
                }
                buffer.swap()
            },
        )
        .flatten();
    allocator.updates.par_extend(updates);
}

pub fn explore_transfers(repository: &Repository, allocator: &mut Allocator) {
    let updates = allocator
        .marked_stops
        .par_iter()
        .enumerate()
        .filter_map(|(i, &m)| m.then_some(i))
        .map_init(
            || LazyBuffer::<Update>::new(32),
            |buffer, stop_idx| {
                // All the possible transfers
                repository.stop_to_transfers[stop_idx]
                    .iter()
                    .for_each(|transfer_idx| {
                        let transfer = &repository.transfers[*transfer_idx as usize];
                        let departure_time =
                            allocator.curr_labels[stop_idx].unwrap_or(u32::MAX.into());
                        let arrival_time = departure_time + transfer_duration(repository, transfer);
                        if arrival_time
                            < allocator.tau_star[transfer.to_stop_idx as usize]
                                .unwrap_or(u32::MAX.into())
                            && arrival_time < allocator.target.tau_star
                        {
                            buffer.push(Update::new(
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
                    });

                let current_stop = &repository.stops[stop_idx];
                repository.stop_to_walk_stop[stop_idx]
                    .iter()
                    .for_each(|next_stop_idx| {
                        let next_stop = &repository.stops[*next_stop_idx as usize];
                        let walking_distance = current_stop
                            .coordinate
                            .network_distance(&next_stop.coordinate);
                        let departure_time =
                            allocator.curr_labels[stop_idx].unwrap_or(u32::MAX.into());
                        let arrival_time = departure_time + time_to_walk(walking_distance);
                        if arrival_time
                            < allocator.tau_star[next_stop.index as usize]
                                .unwrap_or(u32::MAX.into())
                            && arrival_time < allocator.target.tau_star
                        {
                            buffer.push(Update::new(
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
                buffer.swap()
            },
        )
        .flatten();
    allocator.updates.par_extend(updates);
}
