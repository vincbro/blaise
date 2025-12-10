use std::rc::Rc;

use crate::engine::{
    Stop, StopTime,
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
        from_trip_idx: usize,
        to_trip_idx: usize,
    },
    Genesis,
}

pub type NodeRef = Rc<Node>;
#[derive(Debug, Clone)]
pub struct Node {
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
    pub parent: Option<NodeRef>,
}

impl Node {
    pub fn from_coordinate(from: &NodeRef, to: &Stop, end: &NodeRef) -> Self {
        let distance = from.coordinate.distance(&to.coordinate);
        let time_to_walk = time_to_walk(&distance);
        Self {
            stop_idx: Some(to.index),
            coordinate: to.coordinate,
            arrival_time: time_to_walk,
            g_distance: from.g_distance + distance,
            g_time: from.g_time + time_to_walk,
            h_distance: to.coordinate.distance(&end.coordinate),
            transition: Transition::Walk,
            parent: Some(from.clone()),
        }
    }

    pub fn from_stop_time(from: &NodeRef, to: &Stop, stop_time: &StopTime, end: &NodeRef) -> Self {
        Self {
            stop_idx: Some(to.index),
            coordinate: to.coordinate,
            arrival_time: 0,
            g_distance: from.g_distance
                + stop_time
                    .dist_traveled
                    .unwrap_or(from.coordinate.distance(&to.coordinate)),
            g_time: from.g_time, // FIX this should be a real time
            h_distance: to.coordinate.distance(&end.coordinate),
            transition: Transition::Travel {
                trip_idx: stop_time.trip_idx,
                sequence: stop_time.sequence,
            },
            parent: Some(from.clone()),
        }
    }

    pub fn cost(&self) -> u32 {
        (self.h_distance + self.g_distance).as_meters().floor() as u32 + self.g_time as u32
    }
}
impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.cost() == other.cost()
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost().cmp(&self.cost())
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
