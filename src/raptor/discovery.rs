use crate::{
    raptor::{self, Allocator, Location},
    repository::{RaptorRoute, Repository, Stop, Transfer, Trip},
    shared::{AVERAGE_STOP_DISTANCE, Distance, Duration, Time},
};
use tracing::{trace, warn};

pub fn stops_by_location<'a>(
    repository: &'a Repository,
    location: &'a Location,
) -> Result<Vec<&'a Stop>, raptor::Error> {
    match location {
        Location::Area(id) => {
            trace!("Possible area: {id}");
            let area = repository
                .area_by_id(id)
                .ok_or(raptor::Error::InvalidAreaID)?;

            let stops: Vec<_> = repository
                .stops_by_area_idx(area.index)
                .into_iter()
                .filter(|stop| repository.stop_idx_has_trips(stop.index))
                .collect();
            if !stops.is_empty() {
                Ok(stops)
            } else {
                warn!("Had to use coordinates to satisfy stops for area {id}");
                let coordiante = repository.coordinate_by_area_idx(area.index);
                Ok(repository
                    .stops_by_coordinate(&coordiante, AVERAGE_STOP_DISTANCE)
                    .into_iter()
                    .filter(|stop| repository.stop_idx_has_trips(stop.index))
                    .collect())
            }
        }
        Location::Stop(id) => {
            trace!("Possible stop: {id}");
            let stop = repository
                .stop_by_id(id)
                .ok_or(raptor::Error::InvalidStopID)?;
            if let Some(station_idx) = stop.parent_index {
                Ok(repository.stops_by_station(station_idx))
            } else {
                let stops = repository.stops_by_station(stop.index);
                if stops.is_empty() {
                    Ok(vec![stop])
                } else {
                    Ok(stops)
                }
            }
        }
        Location::Coordinate(coordinate) => Ok(repository
            .stops_by_coordinate(coordinate, AVERAGE_STOP_DISTANCE)
            .into_iter()
            .filter(|stop| repository.stop_idx_has_trips(stop.index))
            .collect()),
    }
}

pub(crate) struct ServingRoute {
    pub route_idx: u32,
    pub idx_in_route: u32,
}
pub fn routes_serving_stop(repository: &Repository, stop_idx: u32, allocator: &mut Allocator) {
    allocator.routes_serving_stops.clear();
    allocator.routes_serving_stops.extend(
        repository.stop_to_raptors[stop_idx as usize]
            .iter()
            .filter_map(|route_idx| {
                let route = &repository.raptor_routes[*route_idx as usize];
                index_in_route(route, stop_idx).map(|idx_in_route| ServingRoute {
                    route_idx: route.index,
                    idx_in_route,
                })
            }),
    )
}

pub fn index_in_route(route: &RaptorRoute, stop_idx: u32) -> Option<u32> {
    for (index, route_stop_idx) in route.stops.iter().enumerate() {
        if *route_stop_idx == stop_idx {
            return Some(index as u32);
        }
    }
    None
}

pub fn get_arrival_time(repository: &Repository, trip_idx: u32, index: usize) -> Time {
    let stop_times = repository.stop_times_by_trip_idx(trip_idx);
    stop_times[index].arrival_time
}

pub fn get_departure_time(repository: &Repository, trip_idx: u32, index: usize) -> Time {
    let stop_times = repository.stop_times_by_trip_idx(trip_idx);
    stop_times[index].departure_time
}

/// Finds the earliest trip that we can take from current stop based on the time
pub fn find_earliest_trip<'a>(
    repository: &'a Repository,
    route: &'a RaptorRoute,
    index: usize,
    time: Time,
) -> Option<&'a Trip> {
    let mut earliest: Option<(u32, Time)> = None;

    route
        .trips
        .iter()
        .map(|trip_idx| repository.stop_times_by_trip_idx(*trip_idx))
        .for_each(|stop_times| {
            let stop_time = &stop_times[index];
            let departure_time = stop_time.departure_time;
            // Make sure we don't try to catch a trip that has already left
            if departure_time < time {
                return;
            }
            if let Some((_, time_to_beat)) = earliest {
                if departure_time < time_to_beat {
                    earliest = Some((stop_time.trip_idx, departure_time));
                }
            } else {
                earliest = Some((stop_time.trip_idx, departure_time));
            }
        });

    if let Some((trip_idx, _)) = earliest {
        Some(&repository.trips[trip_idx as usize])
    } else {
        None
    }
}

pub fn transfer_duration<'a>(repository: &'a Repository, transfer: &'a Transfer) -> Duration {
    if let Some(duration) = transfer.min_transfer_time {
        duration
    } else {
        let from = &repository.stops[transfer.from_stop_idx as usize];
        let to = &repository.stops[transfer.to_stop_idx as usize];
        time_to_walk(from.coordinate.network_distance(&to.coordinate))
    }
}

#[inline(always)]
pub const fn time_to_walk(distance: Distance) -> Duration {
    let duration = (distance.as_meters() / 1.5).ceil() as u32;
    Duration::from_seconds(duration)
}
