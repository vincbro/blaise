use std::collections::BinaryHeap;

pub mod graph;
pub mod itinerary;

use thiserror::Error;

use crate::engine::{
    AVERAGE_STOP_DISTANCE, Area, Engine, StopTime,
    geo::{Coordinate, Distance},
    parse_gtfs_time,
    routing::{
        graph::{Location, SearchState, SearchStateRef, Transition},
        itinerary::Itinerary,
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

pub struct Router {
    engine: Engine,
    heap: BinaryHeap<SearchStateRef>,
    best_cost: Vec<usize>,
    from: Location,
    start: SearchStateRef,
    to: Location,
    end: SearchStateRef,
    walk_distance: Distance,
}

impl Router {
    pub fn new(engine: Engine, from: Location, to: Location) -> Result<Self, self::Error> {
        // Build end state
        let end: SearchStateRef = match &to {
            Location::Area(id) => {
                let coordinate = engine
                    .coordinate_by_area_id(id)
                    .ok_or(self::Error::InvalidAreaID)?;
                Ok(SearchState {
                    stop_idx: None,
                    coordinate,
                    current_time: 0,
                    g_distance: Default::default(),
                    g_time: 0,
                    h_distance: Default::default(),
                    penalties: 0,
                    transition: Transition::Genesis,
                    parent: None,
                })
            }
            Location::Stop(id) => {
                let stop = engine.stop_by_id(id).ok_or(self::Error::InvalidStopID)?;
                Ok(SearchState {
                    stop_idx: None,
                    coordinate: stop.coordinate,
                    current_time: 0,
                    g_distance: Default::default(),
                    g_time: 0,
                    h_distance: Default::default(),
                    penalties: 0,
                    transition: Transition::Genesis,
                    parent: None,
                })
            }
            Location::Coordinate(coordinate) => Ok(SearchState {
                stop_idx: None,
                coordinate: *coordinate,
                current_time: 0,
                g_distance: Default::default(),
                g_time: 0,
                h_distance: Default::default(),
                penalties: 0,
                transition: Transition::Genesis,
                parent: None,
            }),
        }?
        .into();

        // Build start state
        let start: SearchStateRef = match &from {
            Location::Area(id) => {
                let coordinate = engine
                    .coordinate_by_area_id(id)
                    .ok_or(self::Error::InvalidAreaID)?;
                let distance = coordinate.distance(&end.coordinate);
                Ok(SearchState {
                    stop_idx: None,
                    coordinate,
                    current_time: parse_gtfs_time("16:00:00").unwrap(),
                    g_distance: Default::default(),
                    g_time: 0,
                    h_distance: distance,
                    penalties: 0,
                    transition: Transition::Genesis,
                    parent: None,
                })
            }
            Location::Stop(id) => {
                let stop = engine.stop_by_id(id).ok_or(self::Error::InvalidStopID)?;
                let distance = stop.coordinate.distance(&end.coordinate);
                Ok(SearchState {
                    stop_idx: None,
                    coordinate: stop.coordinate,
                    current_time: parse_gtfs_time("16:00:00").unwrap(),
                    g_distance: Default::default(),
                    g_time: 0,
                    h_distance: distance,
                    penalties: 0,
                    transition: Transition::Genesis,
                    parent: None,
                })
            }
            Location::Coordinate(coordinate) => Ok(SearchState {
                stop_idx: None,
                coordinate: *coordinate,
                current_time: parse_gtfs_time("16:00:00").unwrap(),
                g_distance: Default::default(),
                g_time: 0,
                h_distance: coordinate.distance(&end.coordinate),
                penalties: 0,
                transition: Transition::Genesis,
                parent: None,
            }),
        }?
        .into();

        Ok(Self {
            best_cost: vec![usize::MAX; engine.stops.len()],
            engine,
            heap: Default::default(),
            walk_distance: AVERAGE_STOP_DISTANCE,
            from,
            start,
            to,
            end,
        })
    }

    pub fn with_walk_distance(mut self, distance: Distance) -> Self {
        self.walk_distance = distance;
        self
    }

    pub fn run(mut self) -> Result<Itinerary, self::Error> {
        // Find all stops close to the start and set them as possible routes
        self.add_walk_neigbours(&self.start.clone());

        while let Some(state) = self.heap.pop() {
            let distance_to_end = self.end.coordinate.distance(&state.coordinate);
            // This is true if we can walk to the end
            if distance_to_end <= self.walk_distance {
                let mut route = vec![self.end];
                let mut next = Some(state);
                while let Some(state) = next {
                    next = state.parent.clone();
                    route.push(state);
                }
                route.reverse();
                return Itinerary::new(self.from, self.to, &route, &self.engine)
                    .ok_or(self::Error::FailedToBuildRoute);
            }
            self.add_neigbours(&state);
            if state.transition != Transition::Walk {
                self.add_walk_neigbours(&state);
            }
        }
        Err(self::Error::NoRouteFound)
    }

    fn add_walk_neigbours(&mut self, node: &SearchStateRef) {
        self.engine
            .stops_by_coordinate(&node.coordinate, self.walk_distance)
            .into_iter()
            .filter(|stop| self.engine.trips_by_stop_id(&stop.id).is_some())
            .for_each(|stop| {
                let node = SearchState::from_coordinate(node, stop, &self.end);
                let cost = node.cost();
                if cost < self.best_cost[stop.index] {
                    self.best_cost[stop.index] = cost;
                    self.heap.push(node.into());
                }
            });
    }

    fn add_neigbours(&mut self, from_node: &SearchStateRef) {
        match from_node.transition {
            Transition::Transit { trip_idx, sequence } => {
                // If we are traveling we will continue down the path
                let trip = &self.engine.trips[trip_idx];
                let stop_times = self.engine.stop_times_by_trip_id(&trip.id).unwrap();
                let mut last_stop_time: Option<&StopTime> = None;
                for stop_time in stop_times.iter() {
                    if stop_time.sequence == sequence {
                        last_stop_time = Some(stop_time);
                        break;
                    }
                }

                if let Some(last_stop_time) = last_stop_time {
                    for new_stop_time in stop_times.iter() {
                        if new_stop_time.sequence != sequence + 1 {
                            continue;
                        }
                        let stop = &self.engine.stops[new_stop_time.stop_idx];
                        let to_node = SearchState::from_stop_time(
                            from_node,
                            stop,
                            last_stop_time,
                            new_stop_time,
                            &self.end,
                        );
                        let cost = to_node.cost();
                        // If it's worth to explore it
                        if cost < self.best_cost[stop.index] {
                            self.best_cost[stop.index] = cost;
                            let node: SearchStateRef = to_node.into();
                            self.heap.push(node.clone());

                            // We also want to explore transfers
                            if let Some(transfers) =
                                self.engine.transfers_by_stop_id(&new_stop_time.stop_id)
                            {
                                transfers.iter().for_each(|transfer| {
                                    let tran_stop = &self.engine.stops[transfer.to_stop_idx];
                                    let tran_node = SearchState::from_transfer(
                                        &node, tran_stop, transfer, &self.end,
                                    );
                                    let cost = tran_node.cost();
                                    if cost < self.best_cost[tran_stop.index] {
                                        self.best_cost[tran_stop.index] = cost;
                                        let t_node: SearchStateRef = tran_node.into();
                                        self.heap.push(t_node);
                                    }
                                });
                            }
                        }
                        break;
                    }
                }
            }
            Transition::Walk => {
                // If the transition was a walk we need to hop on to a transfer
                let stop = &self.engine.stops[from_node.stop_idx.unwrap()];
                self.engine
                    .trips_by_stop_id(&stop.id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|trip| self.engine.stop_times_by_trip_id(&trip.id))
                    .for_each(|stop_times| {
                        let mut last_stop_time: Option<&StopTime> = None;
                        for stop_time in stop_times.iter() {
                            if stop_time.stop_idx == stop.index {
                                last_stop_time = Some(stop_time);
                            }
                        }
                        if let Some(last_stop_time) = last_stop_time {
                            for stop_time in stop_times.iter() {
                                if stop_time.sequence != last_stop_time.sequence + 1 {
                                    continue;
                                }
                                // TEMP should never happend tho
                                let stop = self.engine.stop_by_id(&stop_time.stop_id).unwrap();
                                let node = SearchState::from_stop_time(
                                    from_node,
                                    stop,
                                    last_stop_time,
                                    stop_time,
                                    &self.end,
                                );

                                let cost = node.cost();
                                if cost < self.best_cost[stop.index] {
                                    self.best_cost[stop.index] = cost;
                                    let node: SearchStateRef = node.into();
                                    self.heap.push(node.clone());
                                    // We also want to explore transfers
                                    if let Some(transfers) =
                                        self.engine.transfers_by_stop_id(&stop_time.stop_id)
                                    {
                                        transfers.iter().for_each(|transfer| {
                                            let tran_stop =
                                                &self.engine.stops[transfer.to_stop_idx];
                                            let tran_node = SearchState::from_transfer(
                                                &node, tran_stop, transfer, &self.end,
                                            );
                                            let cost = tran_node.cost();
                                            if cost < self.best_cost[tran_stop.index] {
                                                self.best_cost[tran_stop.index] = cost;
                                                let t_node: SearchStateRef = tran_node.into();
                                                self.heap.push(t_node);
                                            }
                                        });
                                    }
                                }
                                break;
                            }
                        }
                    });
            }
            Transition::Transfer {
                to_stop_idx,
                to_trip_idx,
                ..
            } => {
                match to_trip_idx {
                    Some(to_trip_idx) => {
                        // println!("T_T");
                        // Here we can just start by traveling down the trip
                        let trip = &self.engine.trips[to_trip_idx];
                        let stop_times = self
                            .engine
                            .stop_times_by_trip_id(&trip.id)
                            .unwrap_or_default();
                        let mut last_stop_time: Option<&StopTime> = None;
                        // Find the current sequence id
                        for stop_time in stop_times.iter() {
                            if stop_time.stop_idx == to_stop_idx {
                                last_stop_time = Some(stop_time);
                                break;
                            }
                        }

                        if let Some(last_stop_time) = last_stop_time {
                            for stop_time in stop_times.iter() {
                                if stop_time.sequence != last_stop_time.sequence + 1 {
                                    continue;
                                }
                                let stop = &self.engine.stops[stop_time.stop_idx];
                                let node = SearchState::from_stop_time(
                                    from_node,
                                    stop,
                                    last_stop_time,
                                    stop_time,
                                    &self.end,
                                );
                                let cost = node.cost();
                                // If it's worth to explore it
                                if cost < self.best_cost[stop.index] {
                                    self.best_cost[stop.index] = cost;
                                    let node: SearchStateRef = node.into();
                                    self.heap.push(node.clone());
                                }
                                break;
                            }
                        }
                    }

                    None => {
                        // println!("T_S");
                        let stop = &self.engine.stops[to_stop_idx];
                        self.engine
                            .trips_by_stop_id(&stop.id)
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|trip| self.engine.stop_times_by_trip_id(&trip.id))
                            .for_each(|stop_times| {
                                let mut last_stop_time: Option<&StopTime> = None;
                                // Find the current sequence id
                                for stop_time in stop_times.iter() {
                                    if stop_time.stop_idx != to_stop_idx {
                                        last_stop_time = Some(stop_time);
                                        break;
                                    }
                                }

                                if let Some(last_stop_time) = last_stop_time {
                                    for stop_time in stop_times.iter() {
                                        if stop_time.sequence != last_stop_time.sequence + 1 {
                                            continue;
                                        }
                                        let stop = &self.engine.stops[stop_time.stop_idx];
                                        let node = SearchState::from_stop_time(
                                            from_node,
                                            stop,
                                            last_stop_time,
                                            stop_time,
                                            &self.end,
                                        );
                                        let cost = node.cost();
                                        // If it's worth to explore it and that we can explore
                                        if cost < self.best_cost[stop.index] {
                                            self.best_cost[stop.index] = cost;
                                            let node: SearchStateRef = node.into();
                                            self.heap.push(node.clone());
                                        }
                                        break;
                                    }
                                }
                            });
                    }
                };
            }
            Transition::Genesis => todo!(),
        }
    }
}

pub const fn time_to_walk(distance: &Distance) -> usize {
    // m/s
    const AVERAGE_WALK_SPEED: f64 = 1.5;
    (distance.as_meters() / AVERAGE_WALK_SPEED).ceil() as usize
}
