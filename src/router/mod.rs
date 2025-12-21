use std::collections::BinaryHeap;

pub mod graph;
pub mod itinerary;

use thiserror::Error;

use crate::{
    repository::{Repository, StopTime},
    router::{
        graph::{Location, SearchState, SearchStateRef, Transition},
        itinerary::Itinerary,
    },
    shared::{
        geo::{AVERAGE_STOP_DISTANCE, Coordinate, Distance},
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

pub struct SearchEnv<'a> {
    repo: &'a Repository,
    end: &'a SearchStateRef,
    from: &'a Location,
    to: &'a Location,
    penalty_score: u32,
    walk_distance: Distance,
}

pub struct SearchMut<'a> {
    heap: &'a mut BinaryHeap<SearchStateRef>,
    best_costs: &'a mut Vec<u32>,
}

pub struct Router {
    // State
    repo: Repository,
    heap: BinaryHeap<SearchStateRef>,
    best_costs: Vec<u32>,

    // Conditions
    from: Location,
    to: Location,
    end: SearchStateRef,

    // Options
    walk_distance: Distance,
    penalty_score: u32,
}

impl Router {
    pub fn new(
        repo: Repository,
        from: Location,
        to: Location,
        time: Time,
    ) -> Result<Self, self::Error> {
        let resolve_coord = |loc: &Location| -> Result<Coordinate, self::Error> {
            match loc {
                Location::Area(id) => repo
                    .coordinate_by_area_id(id)
                    .ok_or(self::Error::InvalidAreaID),
                Location::Stop(id) => repo
                    .stop_by_id(id)
                    .map(|stop| stop.coordinate)
                    .ok_or(self::Error::InvalidStopID),
                Location::Coordinate(c) => Ok(*c),
            }
        };

        let end_coordinate = resolve_coord(&to)?;
        let end: SearchStateRef = SearchState {
            stop_idx: None,
            coordinate: end_coordinate,
            current_time: Default::default(),
            g_distance: Default::default(),
            g_time: Default::default(),
            h_distance: Default::default(),
            penalties: 0,
            transition: Transition::Genesis,
            parent: None,
        }
        .into();

        let start_coordinate = resolve_coord(&from)?;
        let start: SearchStateRef = SearchState {
            stop_idx: None,
            coordinate: start_coordinate,
            current_time: time,
            g_distance: Default::default(),
            g_time: Default::default(),
            h_distance: start_coordinate.network_distance(&end.coordinate),
            penalties: 0,
            transition: Transition::Genesis,
            parent: None,
        }
        .into();

        let mut heap = BinaryHeap::new();
        heap.push(start);

        Ok(Self {
            best_costs: vec![u32::MAX; repo.stops.len()],
            repo,
            heap,
            from,
            to,
            end,
            walk_distance: AVERAGE_STOP_DISTANCE,
            penalty_score: 512,
        })
    }

    pub fn with_walk_distance(mut self, distance: Distance) -> Self {
        self.walk_distance = distance;
        self
    }

    pub fn with_penalty_score(mut self, penalty_score: u32) -> Self {
        self.penalty_score = penalty_score;
        self
    }

    pub fn run(mut self) -> Result<Itinerary, self::Error> {
        while let Some(state) = self.heap.pop() {
            let env = SearchEnv {
                repo: &self.repo,
                end: &self.end,
                from: &self.from,
                to: &self.to,
                penalty_score: self.penalty_score,
                walk_distance: self.walk_distance,
            };
            let mut ctx = SearchMut {
                heap: &mut self.heap,
                best_costs: &mut self.best_costs,
            };
            if self.end.coordinate.network_distance(&state.coordinate) <= self.walk_distance {
                return Self::build_itinerary(&env, &state);
            } else {
                Self::add_neighbours(&env, &mut ctx, &state);
            }
        }
        Err(self::Error::NoRouteFound)
    }

    fn build_itinerary(
        env: &SearchEnv,
        from_node: &SearchStateRef,
    ) -> Result<Itinerary, self::Error> {
        let distance_to_end = env.end.coordinate.network_distance(&from_node.coordinate);
        let duration_to_end = time_to_walk(&distance_to_end);
        let end: SearchStateRef = SearchState {
            stop_idx: env.end.stop_idx,
            coordinate: env.end.coordinate,
            current_time: from_node.current_time + duration_to_end,
            g_distance: from_node.g_distance + distance_to_end,
            g_time: from_node.g_time + duration_to_end,
            h_distance: from_node.h_distance + distance_to_end,
            penalties: from_node.penalties,
            transition: Transition::Walk,
            parent: Some(from_node.clone()),
        }
        .into();
        let mut route = vec![];
        let mut next = Some(end);
        while let Some(state) = next {
            next = state.parent.clone();
            route.push(state);
        }
        route.reverse();
        Itinerary::new(env.from.clone(), env.to.clone(), &route, env.repo)
            .ok_or(self::Error::FailedToBuildRoute)
    }

    fn add_walk_neighbours(env: &SearchEnv, ctx: &mut SearchMut, from_node: &SearchStateRef) {
        env.repo
            .stops_by_coordinate(&from_node.coordinate, env.walk_distance)
            .into_iter()
            .filter(|stop| env.repo.trips_by_stop_id(&stop.id).is_some())
            .for_each(|stop| {
                let node =
                    SearchState::from_coordinate(from_node, stop, env.end, env.penalty_score);
                let cost = node.cost();
                if cost < ctx.best_costs[stop.index as usize] {
                    ctx.best_costs[stop.index as usize] = cost;
                    ctx.heap.push(node.into());
                }
            });
    }

    fn add_neighbours(env: &SearchEnv, ctx: &mut SearchMut, from_node: &SearchStateRef) {
        match from_node.transition {
            Transition::Transit { trip_idx, sequence } => {
                Self::explore_trip(env, ctx, from_node, trip_idx, |st| st.sequence == sequence);
                Self::add_walk_neighbours(env, ctx, from_node);
            }

            Transition::Walk => {
                if let Some(stop_idx) = from_node.stop_idx {
                    Self::explore_stop_trips(env, ctx, from_node, stop_idx);
                }
            }
            Transition::Transfer {
                to_stop_idx,
                to_trip_idx,
                ..
            } => match to_trip_idx {
                Some(trip_idx) => {
                    Self::explore_trip(env, ctx, from_node, trip_idx, |st| {
                        st.stop_idx == to_stop_idx
                    });
                    Self::add_walk_neighbours(env, ctx, from_node);
                }

                None => {
                    Self::explore_stop_trips(env, ctx, from_node, to_stop_idx);
                    Self::add_walk_neighbours(env, ctx, from_node);
                }
            },
            Transition::Genesis => Self::add_walk_neighbours(env, ctx, from_node),
        }
    }

    fn explore_stop_trips(
        env: &SearchEnv,
        ctx: &mut SearchMut,
        from_node: &SearchStateRef,
        stop_idx: u32,
    ) {
        let stop = &env.repo.stops[stop_idx as usize];
        if let Some(trips) = env.repo.trips_by_stop_id(&stop.id) {
            for trip in trips {
                Self::explore_trip(env, ctx, from_node, trip.index, |st| {
                    st.stop_idx == stop_idx
                });
            }
        }
    }

    fn explore_trip(
        env: &SearchEnv,
        ctx: &mut SearchMut,
        from_node: &SearchStateRef,
        trip_idx: u32,
        find_current: impl Fn(&StopTime) -> bool,
    ) {
        let trip = env.repo.trips[trip_idx as usize].clone();
        let stop_times = env.repo.stop_times_by_trip_id(&trip.id).unwrap();

        if let Some(last_stop_time) = stop_times.iter().find(|st| find_current(st))
            && let Some(next_stop_time) = stop_times
                .iter()
                .find(|st| st.sequence == last_stop_time.sequence + 1)
        {
            Self::process_segment(env, ctx, from_node, last_stop_time, next_stop_time);
        }
    }

    fn process_segment(
        env: &SearchEnv,
        ctx: &mut SearchMut,
        from_node: &SearchStateRef,
        last_stop_time: &StopTime,
        new_stop_time: &StopTime,
    ) {
        let stop = &env.repo.stops[new_stop_time.stop_idx as usize];
        let to_node = SearchState::from_stop_time(
            from_node,
            stop,
            last_stop_time,
            new_stop_time,
            env.end,
            env.penalty_score,
        );

        let cost = to_node.cost();
        if cost < ctx.best_costs[stop.index as usize] {
            ctx.best_costs[stop.index as usize] = cost;
            let node: SearchStateRef = to_node.into();
            ctx.heap.push(node.clone());
            Self::explore_transfers(env, ctx, &node, &new_stop_time.stop_id);
        }
    }

    fn explore_transfers(
        env: &SearchEnv,
        ctx: &mut SearchMut,
        from_node: &SearchStateRef,
        stop_id: &str,
    ) {
        if let Some(transfers) = env.repo.transfers_by_stop_id(stop_id) {
            for transfer in transfers {
                let tran_stop = &env.repo.stops[transfer.to_stop_idx as usize];
                let tran_node = SearchState::from_transfer(
                    from_node,
                    tran_stop,
                    transfer,
                    env.end,
                    env.penalty_score,
                );

                let cost = tran_node.cost();
                if cost < ctx.best_costs[tran_stop.index as usize] {
                    ctx.best_costs[tran_stop.index as usize] = cost;
                    ctx.heap.push(tran_node.into());
                }
            }
        }
    }
}

pub(crate) const fn time_to_walk(distance: &Distance) -> Duration {
    // m/s
    const AVERAGE_WALK_SPEED: f32 = 1.5;
    Duration::from_seconds((distance.as_meters() / AVERAGE_WALK_SPEED).ceil() as u32)
}
