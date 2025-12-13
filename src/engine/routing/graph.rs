use std::rc::Rc;

use crate::engine::{
    Stop, StopTime, Transfer,
    geo::{Coordinate, Distance},
    routing::time_to_walk,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Transition {
    Travel {
        trip_idx: usize,
        sequence: usize,
    },
    Walk,
    Transfer {
        from_stop_idx: usize,
        to_stop_idx: usize,
        to_trip_idx: Option<usize>,
    },
    Genesis,
}

pub type SearchStateRef = Rc<SearchState>;
#[derive(Debug, Clone)]
pub struct SearchState {
    pub stop_idx: Option<usize>,
    pub coordinate: Coordinate,
    pub arrival_time: usize,
    // The distance we have traveled
    pub g_distance: Distance,
    // The time we have traveld
    pub g_time: usize,
    // The distance we still need to travel
    pub h_distance: Distance,
    pub transition: Transition,
    pub parent: Option<SearchStateRef>,
}

impl SearchState {
    pub fn from_coordinate(from: &SearchStateRef, to: &Stop, end: &SearchStateRef) -> Self {
        let distance = from.coordinate.distance(&to.coordinate);
        let time_to_walk = time_to_walk(&distance);
        Self {
            stop_idx: Some(to.index),
            coordinate: to.coordinate,
            arrival_time: from.arrival_time + time_to_walk,
            g_distance: from.g_distance + distance,
            g_time: from.g_time + time_to_walk,
            h_distance: to.coordinate.distance(&end.coordinate),
            transition: Transition::Walk,
            parent: Some(from.clone()),
        }
    }

    pub fn from_transfer(
        from: &SearchStateRef,
        to: &Stop,
        transfer: &Transfer,
        end: &SearchStateRef,
    ) -> Self {
        let distance = from.coordinate.distance(&to.coordinate);
        let time_to_transfer = transfer.min_transfer_time.unwrap_or(0) + time_to_walk(&distance);
        Self {
            stop_idx: Some(to.index),
            coordinate: to.coordinate,
            arrival_time: from.arrival_time + time_to_transfer,
            g_distance: from.g_distance + distance,
            g_time: from.g_time + time_to_transfer,
            h_distance: to.coordinate.distance(&end.coordinate),
            transition: Transition::Transfer {
                from_stop_idx: transfer.from_stop_idx,
                to_stop_idx: transfer.to_stop_idx,
                to_trip_idx: transfer.to_trip_idx,
            },
            parent: Some(from.clone()),
        }
    }

    pub fn from_stop_time(
        from: &SearchStateRef,
        to: &Stop,
        stop_time: &StopTime,
        end: &SearchStateRef,
    ) -> Self {
        Self {
            stop_idx: Some(to.index),
            coordinate: to.coordinate,
            arrival_time: stop_time.departure_time,
            g_distance: from.g_distance
                + stop_time
                    .dist_traveled
                    .unwrap_or(from.coordinate.distance(&to.coordinate)),
            g_time: from.g_time + stop_time.departure_time - from.arrival_time,
            h_distance: to.coordinate.distance(&end.coordinate),
            transition: Transition::Travel {
                trip_idx: stop_time.trip_idx,
                sequence: stop_time.sequence,
            },
            parent: Some(from.clone()),
        }
    }

    pub fn cost(&self) -> usize {
        // (self.h_distance + self.g_distance).as_meters().floor() as u32 + self.g_time as u32
        self.h_distance.as_meters().floor() as usize + self.g_time
    }
}
impl Eq for SearchState {}

impl PartialEq for SearchState {
    fn eq(&self, other: &Self) -> bool {
        self.cost() == other.cost()
    }
}

impl Ord for SearchState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost().cmp(&self.cost())
    }
}

impl PartialOrd for SearchState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
