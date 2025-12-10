use std::{collections::BinaryHeap, sync::Arc};

pub mod graph;

use thiserror::Error;

use crate::engine::{
    AVERAGE_STOP_DISTANCE, Engine, Stop, StopTime,
    geo::{Coordinate, Distance},
    routing::graph::{Node, NodeRef, Transition},
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Stop id does not match any entry")]
    InvalidStopID,
    #[error("Could not find a route")]
    NoRouteFound,
}

#[derive(Debug, Clone)]
pub enum Waypoint {
    Stop(Arc<str>),
    Coordinate(Coordinate),
}

impl From<&Stop> for Waypoint {
    fn from(value: &Stop) -> Self {
        Self::Stop(value.id.clone())
    }
}

impl From<Stop> for Waypoint {
    fn from(value: Stop) -> Self {
        Self::Stop(value.id)
    }
}

impl From<Coordinate> for Waypoint {
    fn from(value: Coordinate) -> Self {
        Self::Coordinate(value)
    }
}

pub struct Router {
    engine: Engine,
    heap: BinaryHeap<NodeRef>,
    best_cost: Vec<u32>,
    start: NodeRef,
    end: NodeRef,
    walk_distance: Distance,
}

impl Router {
    pub fn new(engine: Engine, from: Waypoint, to: Waypoint) -> Result<Self, self::Error> {
        // Build end node
        let end: NodeRef = match to {
            Waypoint::Stop(id) => {
                let stop = engine.stop_by_id(&id).ok_or(self::Error::InvalidStopID)?;
                Ok(Node {
                    stop_idx: Some(stop.index),
                    coordinate: stop.coordinate,
                    arrival_time: 0,
                    g_distance: Default::default(),
                    g_time: 0,
                    h_distance: Default::default(),
                    transition: Transition::Genesis,
                    parent: None,
                })
            }
            Waypoint::Coordinate(coordinate) => Ok(Node {
                stop_idx: None,
                coordinate,
                arrival_time: 0,
                g_distance: Default::default(),
                g_time: 0,
                h_distance: Default::default(),
                transition: Transition::Genesis,
                parent: None,
            }),
        }?
        .into();

        // Build start node
        let start: NodeRef = match from {
            Waypoint::Stop(id) => {
                let stop = engine.stop_by_id(&id).ok_or(self::Error::InvalidStopID)?;
                let distance = stop.coordinate.distance(&end.coordinate);
                Ok(Node {
                    stop_idx: Some(stop.index),
                    coordinate: stop.coordinate,
                    arrival_time: 0,
                    g_distance: Default::default(),
                    g_time: 0,
                    h_distance: distance,
                    transition: Transition::Genesis,
                    parent: None,
                })
            }
            Waypoint::Coordinate(coordinate) => Ok(Node {
                stop_idx: None,
                coordinate,
                arrival_time: 0,
                g_distance: Default::default(),
                g_time: 0,
                h_distance: coordinate.distance(&end.coordinate),
                transition: Transition::Genesis,
                parent: None,
            }),
        }?
        .into();

        Ok(Self {
            best_cost: vec![u32::MAX; engine.stops.len()],
            engine,
            heap: Default::default(),
            walk_distance: AVERAGE_STOP_DISTANCE,
            start,
            end,
        })
    }

    pub fn with_walk_distance(mut self, distance: Distance) -> Self {
        self.walk_distance = distance;
        self
    }

    pub fn run(&mut self) -> Result<Vec<String>, self::Error> {
        // Find all stops close to the start and set them as possible routes
        self.engine
            .stops_by_coordinate(&self.start.coordinate, self.walk_distance)
            .into_iter()
            .filter(|stop| self.engine.trips_by_stop_id(&stop.id).is_some())
            .for_each(|stop| {
                let node = Node::from_coordinate(&self.start, stop, &self.end);
                let cost = node.cost();
                if cost < self.best_cost[stop.index] {
                    self.best_cost[stop.index] = cost;
                    self.heap.push(node.into());
                }
            });

        while let Some(node) = self.heap.pop() {
            let distance_to_end = self.end.coordinate.distance(&node.coordinate);
            // This is true if we can walk to the end
            if distance_to_end <= self.walk_distance {
                let mut route: Vec<String> = vec![];
                route.push(self.node_to_str(&self.end));
                let mut walk = Some(node);
                while let Some(node) = walk {
                    route.push(self.node_to_str(&node));
                    walk = node.parent.clone();
                }
                route.reverse();
                return Ok(route);
            }
            // if let Some(stop_idx) = node.stop_idx {
            //     // println!("name: {}", self.engine.stops[stop_idx].name);
            // }
            self.add_neigbours(node);
        }
        Err(self::Error::NoRouteFound)
    }

    fn add_neigbours(&mut self, node: NodeRef) {
        match node.transition {
            Transition::Travel { trip_idx, sequence } => {
                // If we are traveling we will continue down the path
                let trip = &self.engine.trips[trip_idx];
                let stop_times = self.engine.stop_times_by_trip_id(&trip.id).unwrap();
                for stop_time in stop_times {
                    if stop_time.sequence == sequence + 1 {
                        let stop = self.engine.stop_by_id(&stop_time.stop_id).unwrap();
                        let node = Node::from_stop_time(&node, stop, stop_time, &self.end);
                        let cost = node.cost();
                        if cost < self.best_cost[stop.index] {
                            self.best_cost[stop.index] = cost;
                            self.heap.push(node.into());
                        }
                        break;
                    }
                }
                // We also want to try to
            }
            Transition::Walk => {
                // If the transition was a walk we need to hop on to a transfer
                let stop = &self.engine.stops[node.stop_idx.unwrap()];
                self.engine
                    .trips_by_stop_id(&stop.id)
                    .unwrap()
                    .into_iter()
                    .filter_map(|trip| self.engine.stop_times_by_trip_id(&trip.id))
                    .for_each(|stop_times| {
                        let mut from_stop_time: Option<&StopTime> = None;
                        for stop_time in stop_times.into_iter() {
                            if from_stop_time.is_none() && stop_time.stop_idx == stop.index {
                                from_stop_time = Some(stop_time);
                            } else if from_stop_time.is_some() {
                                // TEMP should never happend tho
                                let stop = self.engine.stop_by_id(&stop_time.stop_id).unwrap();
                                let node =
                                    Node::from_stop_time(&self.start, stop, stop_time, &self.end);

                                let cost = node.cost();
                                if cost < self.best_cost[stop.index] {
                                    self.best_cost[stop.index] = cost;
                                    self.heap.push(node.into());
                                }
                                break;
                            }
                        }
                    });
            }
            Transition::Transfer {
                from_trip_idx,
                to_trip_idx,
            } => todo!(),
            Transition::Genesis => todo!(),
        }
    }

    fn node_to_str(&self, node: &NodeRef) -> String {
        let prefix = match node.transition {
            Transition::Travel { .. } => format!("Travel to"),
            Transition::Walk => format!("Walk to"),
            Transition::Transfer { .. } => format!("Transfer to"),
            Transition::Genesis => format!("START/END from"),
        };
        let name = match node.stop_idx {
            Some(stop_idx) => format!("{}", self.engine.stops[stop_idx].name),
            None => format!(
                "Position: {}, {}",
                node.coordinate.latitude, node.coordinate.longitude
            ),
        };
        format!("{} {}", prefix, name)
    }
}

pub const fn time_to_walk(distance: &Distance) -> usize {
    // m/s
    const AVERAGE_WALK_SPEED: f64 = 1.5;
    (distance.as_meters() / AVERAGE_WALK_SPEED).ceil() as usize
}
