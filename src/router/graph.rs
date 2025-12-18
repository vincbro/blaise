use std::{rc::Rc, sync::Arc};

use crate::{
    repository::{Area, Stop, StopTime, Transfer},
    router::time_to_walk,
    shared::{
        geo::{Coordinate, Distance},
        time::{Duration, Time},
    },
};

#[derive(Debug, Clone)]
pub enum Location {
    Area(Arc<str>),
    Stop(Arc<str>),
    Coordinate(Coordinate),
}

impl From<&Area> for Location {
    fn from(value: &Area) -> Self {
        Self::Area(value.id.clone())
    }
}

impl From<Area> for Location {
    fn from(value: Area) -> Self {
        Self::Area(value.id)
    }
}

impl From<Coordinate> for Location {
    fn from(value: Coordinate) -> Self {
        Self::Coordinate(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Transition {
    Transit {
        trip_idx: u32,
        sequence: u16,
    },
    Walk,
    Transfer {
        from_stop_idx: u32,
        to_stop_idx: u32,
        to_trip_idx: Option<u32>,
    },
    Genesis,
}

impl Transition {
    pub fn switch_cost(&self, other: &Self) -> bool {
        use Transition::*;
        matches!(
            (self, other),
            (Transit { .. }, Walk)
                | (Transit { .. }, Transfer { .. })
                | (Walk, Walk)
                | (Transfer { .. }, Transfer { .. })
                | (Transfer { .. }, Walk)
                | (Walk, Transfer { .. })
        )
    }

    pub fn is_same_leg(&self, other: &Self) -> bool {
        self.inner_is_same_leg(other) || other.inner_is_same_leg(self)
    }

    fn inner_is_same_leg(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Genesis, Self::Walk) => true,
            // (Self::Transit { .. }, Self::Walk) => true,
            (Self::Transit { trip_idx: t1, .. }, Self::Transit { trip_idx: t2, .. }) => t1 == t2,
            _ => false,
        }
    }
}

pub type SearchStateRef = Rc<SearchState>;
#[derive(Debug, Clone)]
pub struct SearchState {
    pub stop_idx: Option<u32>,
    pub coordinate: Coordinate,
    pub current_time: Time,
    // The distance we have traveled
    pub g_distance: Distance,
    // The time we have traveld
    pub g_time: Duration,
    // The distance we still need to travel
    pub h_distance: Distance,
    // This allows us to modify how harshly we look at transfers and walking
    pub penalties: u32,
    pub transition: Transition,
    pub parent: Option<SearchStateRef>,
}

impl SearchState {
    pub fn from_coordinate(
        from: &SearchStateRef,
        to: &Stop,
        end: &SearchStateRef,
        p_score: u32,
    ) -> Self {
        let distance = from.coordinate.network_distance(&to.coordinate);
        let time_to_walk = time_to_walk(&distance);

        let penalty = if from.transition.switch_cost(&Transition::Walk) {
            p_score
        } else {
            0
        };
        Self {
            stop_idx: Some(to.index),
            coordinate: to.coordinate,
            current_time: from.current_time + time_to_walk,
            g_distance: from.g_distance + distance,
            g_time: from.g_time + time_to_walk,
            h_distance: to.coordinate.network_distance(&end.coordinate),
            penalties: from.penalties + penalty,
            transition: Transition::Walk,
            parent: Some(from.clone()),
        }
    }

    pub fn from_transfer(
        from: &SearchStateRef,
        to: &Stop,
        transfer: &Transfer,
        end: &SearchStateRef,
        p_score: u32,
    ) -> Self {
        let distance = from.coordinate.network_distance(&to.coordinate);
        let time_to_transfer =
            transfer.min_transfer_time.unwrap_or_default() + time_to_walk(&distance);

        let transition = Transition::Transfer {
            from_stop_idx: transfer.from_stop_idx,
            to_stop_idx: transfer.to_stop_idx,
            to_trip_idx: transfer.to_trip_idx,
        };

        let penalty = if from.transition.switch_cost(&transition) {
            p_score
        } else {
            0
        };

        Self {
            stop_idx: Some(to.index),
            coordinate: to.coordinate,
            current_time: from.current_time + time_to_transfer,
            g_distance: from.g_distance + distance,
            g_time: from.g_time + time_to_transfer,
            h_distance: to.coordinate.network_distance(&end.coordinate),
            penalties: from.penalties + penalty,
            transition,
            parent: Some(from.clone()),
        }
    }

    pub fn from_stop_time(
        from: &SearchStateRef,
        to: &Stop,
        last_stop_time: &StopTime, // Stop we just left
        new_stop_time: &StopTime,  // The stop we will arrive at
        end: &SearchStateRef,
        p_score: u32,
    ) -> Self {
        let mut boarding_time = last_stop_time.departure_time;
        if boarding_time < from.current_time {
            boarding_time += Duration::from_days(1); // The train leaves "tomorrow" relative to previous arrival
        }

        // 2. Calculate Trip Duration (handling midnight crossing on the train)
        let raw_departure = last_stop_time.departure_time;
        let mut raw_arrival = new_stop_time.arrival_time;

        // Fix messy GTFS data where a trip goes 23:50 -> 00:10 without marking it as 24:10 (gtfs should account for this btw)
        if raw_arrival < raw_departure {
            raw_arrival += Duration::from_days(1);
        }
        let travel_duration = raw_arrival - raw_departure;
        let arrival_time = boarding_time + travel_duration;

        let dist_delta = match (new_stop_time.dist_traveled, last_stop_time.dist_traveled) {
            (Some(new_dist), Some(old_dist)) => new_dist - old_dist,
            _ => from.coordinate.network_distance(&to.coordinate),
        };
        // let dist_delta = from.coordinate.network_distance(&to.coordinate);

        let transition = Transition::Transit {
            trip_idx: new_stop_time.trip_idx,
            sequence: new_stop_time.sequence,
        };

        let penalty = if from.transition.switch_cost(&transition) {
            p_score
        } else {
            0
        };

        Self {
            stop_idx: Some(to.index),
            coordinate: to.coordinate,
            current_time: arrival_time,
            g_distance: from.g_distance + dist_delta,
            g_time: from.g_time + (arrival_time - from.current_time),
            h_distance: to.coordinate.network_distance(&end.coordinate),
            penalties: from.penalties + penalty,
            transition,
            parent: Some(from.clone()),
        }
    }

    pub fn cost(&self) -> u32 {
        // 28 m/s is roughly 100 km/h
        const MAX_TRANSIT_SPEED: f32 = 28.0;
        let h_time =
            Duration::from_seconds((self.h_distance.as_meters() / MAX_TRANSIT_SPEED) as u32);
        let cost = self.g_time + h_time + Duration::from_seconds(self.penalties);
        cost.as_seconds()
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
