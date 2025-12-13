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
    pub current_time: usize,
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
            current_time: from.current_time + time_to_walk,
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
            current_time: from.current_time + time_to_transfer,
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
        last_stop_time: &StopTime, // Stop we just left
        new_stop_time: &StopTime,  // The stop we will arrive at
        end: &SearchStateRef,
    ) -> Self {
        let mut boarding_time = last_stop_time.departure_time;
        if boarding_time < from.current_time {
            boarding_time += 86400; // The train leaves "tomorrow" relative to previous arrival
        }

        // 2. Calculate Trip Duration (handling midnight crossing on the train)
        let raw_departure = last_stop_time.departure_time;
        let mut raw_arrival = new_stop_time.arrival_time;

        // Fix messy GTFS data where a trip goes 23:50 -> 00:10 without marking it as 24:10 (gtfs should account for this btw)
        if raw_arrival < raw_departure {
            raw_arrival += 86400;
        }
        let travel_duration = raw_arrival - raw_departure;
        let arrival_time = boarding_time + travel_duration;

        let dist_delta = match (new_stop_time.dist_traveled, last_stop_time.dist_traveled) {
            (Some(new_dist), Some(old_dist)) => new_dist - old_dist,
            _ => from.coordinate.distance(&to.coordinate),
        };

        Self {
            stop_idx: Some(to.index),
            coordinate: to.coordinate,
            current_time: arrival_time,
            g_distance: dist_delta,
            g_time: from.g_time + (arrival_time - from.current_time),
            h_distance: to.coordinate.distance(&end.coordinate),
            transition: Transition::Travel {
                trip_idx: new_stop_time.trip_idx,
                sequence: new_stop_time.sequence,
            },
            parent: Some(from.clone()),
        }
    }

    pub fn cost(&self) -> usize {
        // (self.h_distance + self.g_distance).as_meters().floor() as usize + self.g_time
        // self.h_distance.as_meters().floor() as usize + self.g_time

        // 1. Heuristic Time (H)
        // We convert the remaining distance to time using a fast speed (e.g. ~100 km/h).
        // This ensures the heuristic is 'admissible' (never overestimates the cost),
        // which is required for A* to find the optimal path.
        const MAX_TRANSIT_SPEED: f64 = 28.0; // 28 m/s is roughly 100 km/h
        let h_time = (self.h_distance.as_meters() / MAX_TRANSIT_SPEED) as usize;

        // 2. Base Cost: Total Time (G + H)
        // This makes "Faster" the primary goal.
        let time_cost = self.g_time + h_time;

        // 3. Distance Penalty (Tie-Breaker for "Shorter")
        // We add a tiny cost for distance to favor shorter routes when times are similar.
        // e.g., 0.01 means 1 km of travel adds 10 seconds of "virtual cost".
        // This prevents the router from taking a massive detour just to save 1 second.
        let distance_penalty = (self.g_distance.as_meters() * 0.01) as usize;

        time_cost + distance_penalty
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
