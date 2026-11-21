use flatnet_core::{
    consumer::Consumer,
    models::{BoardingRule, Duration, Optional, StopIndex, Time, TripIndex, TripPatternIndex},
};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{
    raptor::{
        self, Day, LiveTime, PatternPosition,
        pareto::{self, ParetoLabel},
        query::{RaptorQuery, StopTime},
        state::State,
        update::{Parent, StepKind, Update},
    },
    spatial::SpatialGrid,
};

/// The updates one worker thread expects to find, so its buffer rarely grows.
const BUFFER_CAPACITY: usize = 512;

/// A trip being ridden while a pattern is scanned, with the position it was boarded at going
/// forward or left at going back, and what it cost to get there.
#[derive(Debug, Clone, Copy)]
struct Ride {
    trip: TripIndex,
    day: Day,
    position: PatternPosition,
    cost: f64,
}

/// A move from the stop a label is at to another stop on foot or by a listed transfer.
#[derive(Debug, Clone, Copy)]
struct Hop {
    from: StopIndex,
    to: StopIndex,
    seconds: u32,
    penalty: f64,
    kind: StepKind,
}

/// Finds the labels one more round of the search reaches, from what the last round found.
#[derive(Debug)]
pub(super) struct Explorer<'a> {
    query: &'a RaptorQuery<'a>,
    consumer: &'a Consumer<'a>,
    spatial: &'a SpatialGrid,
}

impl<'a> Explorer<'a> {
    pub(super) const fn new(
        query: &'a RaptorQuery<'a>,
        consumer: &'a Consumer<'a>,
        spatial: &'a SpatialGrid,
    ) -> Self {
        Self {
            query,
            consumer,
            spatial,
        }
    }

    /// Rides every trip pattern from where the last round marked a stop on it.
    pub(super) fn trip_patterns(&self, state: &State) -> Vec<Update> {
        state
            .active_trip_patterns()
            .par_iter()
            .copied()
            .enumerate()
            .filter_map(|(pattern, start)| Some((pattern, start.as_option()?)))
            .fold(
                || Vec::with_capacity(BUFFER_CAPACITY),
                |mut buffer, (pattern, start)| {
                    let pattern = TripPatternIndex::from(pattern);
                    if self.query.is_arrival() {
                        self.scan_back(state, pattern, start, &mut buffer);
                    } else {
                        self.scan_forward(state, pattern, start, &mut buffer);
                    }
                    buffer
                },
            )
            .reduce(Vec::new, merge_buffers)
    }

    /// Moves from every stop the last step marked to the stops a transfer or a walk reaches.
    pub(super) fn transfers(&self, state: &State) -> Vec<Update> {
        state
            .marked_stops()
            .par_iter()
            .enumerate()
            .filter(|&(_, &marked)| marked)
            .fold(
                || Vec::with_capacity(BUFFER_CAPACITY),
                |mut buffer, (stop, _)| {
                    self.transfers_from(state, StopIndex::from(stop), &mut buffer);
                    buffer
                },
            )
            .reduce(Vec::new, merge_buffers)
    }

    /// Moves from one stop with every label it has this round.
    fn transfers_from(&self, state: &State, stop: StopIndex, buffer: &mut Vec<Update>) {
        for label in state.current_labels(stop).iter() {
            if self.query.is_arrival() {
                self.transfers_back(state, stop, label, buffer);
            } else {
                self.transfers_forward(state, stop, label, buffer);
            }
        }
    }

    /// Rides forward along a pattern, boarding wherever a label gets to a trip sooner than the one
    /// already ridden, and emitting a label at every stop it can alight at.
    fn scan_forward(
        &self,
        state: &State,
        pattern: TripPatternIndex,
        start: PatternPosition,
        buffer: &mut Vec<Update>,
    ) {
        let stops = self.consumer.stop_sequence_by_trip_pattern(pattern);
        let mut ride: Option<Ride> = None;
        for (index, &stop) in stops.iter().enumerate().skip(start.as_usize()) {
            let position = PatternPosition::from(index);
            if let Some(ride) = ride {
                self.alight(state, ride, stop, position, buffer);
            }
            for label in state.previous_labels(stop).iter() {
                ride = self.board(ride, label, pattern, position).or(ride);
            }
        }
    }

    /// Rides back along a pattern, the mirror of [`Self::scan_forward`]: it picks the latest trip
    /// that gets to a label in time, and emits a label at every stop it can be boarded at.
    fn scan_back(
        &self,
        state: &State,
        pattern: TripPatternIndex,
        start: PatternPosition,
        buffer: &mut Vec<Update>,
    ) {
        let stops = self.consumer.stop_sequence_by_trip_pattern(pattern);
        let mut ride: Option<Ride> = None;
        for (index, &stop) in stops[..=start.as_usize()].iter().enumerate().rev() {
            let position = PatternPosition::from(index);
            if let Some(ride) = ride {
                self.board_back(state, ride, stop, position, buffer);
            }
            for label in state.previous_labels(stop).iter() {
                ride = self.alight_back(ride, label, pattern, position).or(ride);
            }
        }
    }

    /// Emits the label for getting off `ride` at `stop`.
    fn alight(
        &self,
        state: &State,
        ride: Ride,
        stop: StopIndex,
        position: PatternPosition,
        buffer: &mut Vec<Update>,
    ) {
        let stop_time = self.stop_time(ride.trip, position);
        let day = ride.day;
        if stop_time.drop_off == BoardingRule::NOT_AVAILABLE {
            return;
        }
        let Some(arrival) = self.query.arrival(&stop_time, day) else {
            return;
        };
        if state.is_dominated_at(stop, arrival.actual, pareto::narrow_cost(ride.cost), false) {
            return;
        }
        let Some(departure) = self.departure(ride.trip, day, ride.position) else {
            return;
        };

        // A feed with delays can put a stop's arrival before the departure it follows.
        let ride_time = f64::from(arrival.actual.0.saturating_sub(departure.actual.0));
        let cost = pareto::narrow_cost(ride.cost + ride_time * self.query.transit_penalty);
        if state.is_dominated(stop, arrival.actual, cost, false) {
            return;
        }
        buffer.push(Update::new(
            stop,
            arrival.actual,
            cost,
            Parent::Transit {
                boarding_position: ride.position,
                alighting_position: position,
                trip: ride.trip,
                day,
                departure_time: departure,
                arrival_time: arrival,
            },
        ));
    }

    /// Emits the label for getting on `ride` at `stop`, when searching back from an arrival.
    fn board_back(
        &self,
        state: &State,
        ride: Ride,
        stop: StopIndex,
        position: PatternPosition,
        buffer: &mut Vec<Update>,
    ) {
        let stop_time = self.stop_time(ride.trip, position);
        let day = ride.day;
        if stop_time.pickup == BoardingRule::NOT_AVAILABLE {
            return;
        }
        let Some(departure) = self.query.departure(&stop_time, day) else {
            return;
        };
        if state.is_dominated_at(stop, departure.actual, pareto::narrow_cost(ride.cost), true) {
            return;
        }
        let Some(arrival) = self.arrival(ride.trip, day, ride.position) else {
            return;
        };

        // A feed with delays can put a stop's arrival before the departure it follows.
        let ride_time = f64::from(arrival.actual.0.saturating_sub(departure.actual.0));
        let cost = pareto::narrow_cost(ride.cost + ride_time * self.query.transit_penalty);
        if state.is_dominated(stop, departure.actual, cost, true) {
            return;
        }
        buffer.push(Update::new(
            stop,
            departure.actual,
            cost,
            Parent::Transit {
                boarding_position: position,
                alighting_position: ride.position,
                trip: ride.trip,
                day,
                departure_time: departure,
                arrival_time: arrival,
            },
        ));
    }

    /// The ride a label leads to if it gets to a trip sooner than the one already ridden.
    fn board(
        &self,
        ride: Option<Ride>,
        label: ParetoLabel,
        pattern: TripPatternIndex,
        position: PatternPosition,
    ) -> Option<Ride> {
        let current_departure = ride
            .and_then(|ride| self.departure(ride.trip, ride.day, position))
            .map_or(Time(u32::MAX), |departure| departure.actual);
        if label.time >= current_departure {
            return None;
        }

        let (trip, day) = self.find_earliest_trip(pattern, position, label.time)?;
        let candidate = self.departure(trip, day, position)?;
        let wait = f64::from(candidate.actual.0 - label.time.0);
        let cost = f64::from(label.cost) + wait * self.query.transit_penalty;

        // An earlier departure wins, and so does a cheaper way to the same one.
        let better = candidate.actual < current_departure
            || (candidate.actual == current_departure
                && cost < ride.map_or(f64::MAX, |ride| ride.cost));
        better.then_some(Ride {
            trip,
            day,
            position,
            cost,
        })
    }

    /// The ride a label leads to if it leaves a trip later than the one already ridden.
    fn alight_back(
        &self,
        ride: Option<Ride>,
        label: ParetoLabel,
        pattern: TripPatternIndex,
        position: PatternPosition,
    ) -> Option<Ride> {
        let current_arrival = ride
            .and_then(|ride| self.arrival(ride.trip, ride.day, position))
            .map_or(Time(0), |arrival| arrival.actual);
        if label.time < current_arrival {
            return None;
        }

        let (trip, day) = self.find_latest_trip(pattern, position, label.time)?;
        let candidate = self.arrival(trip, day, position)?;
        let wait = f64::from(label.time.0 - candidate.actual.0);
        let cost = f64::from(label.cost) + wait * self.query.transit_penalty;

        let better = candidate.actual > current_arrival
            || ride.is_none_or(|ride| candidate.actual == current_arrival && cost < ride.cost);
        better.then_some(Ride {
            trip,
            day,
            position,
            cost,
        })
    }

    /// The trips of a pattern that run on one of the days the search covers, each with the day.
    fn runs(&self, pattern: TripPatternIndex) -> impl Iterator<Item = (TripIndex, Day)> + 'a {
        let (consumer, query) = (self.consumer, self.query);
        let trips = consumer.iter_trips_in_trip_pattern(pattern);
        trips.flat_map(move |trip| {
            Day::ALL.into_iter().filter_map(move |day| {
                let date = query.service_date(day)?;
                consumer
                    .is_service_active(trip.service_index, date)
                    .then_some((trip.index, day))
            })
        })
    }

    /// The first trip of the pattern a passenger ready at `ready_time` can board at `position`,
    /// and the day it runs on.
    fn find_earliest_trip(
        &self,
        pattern: TripPatternIndex,
        position: PatternPosition,
        ready_time: Time,
    ) -> Option<(TripIndex, Day)> {
        self.runs(pattern)
            .filter_map(|(trip, day)| {
                let stop_time = self.stop_time_in(pattern, trip, position);
                if stop_time.pickup == BoardingRule::NOT_AVAILABLE {
                    return None;
                }
                let departure = self.query.departure(&stop_time, day)?;
                Some((trip, day, departure.actual))
            })
            .filter(|&(_, _, departure)| departure >= ready_time)
            .min_by_key(|&(_, _, departure)| departure)
            .map(|(trip, day, _)| (trip, day))
    }

    /// The last trip of the pattern that gets to `position` by `ready_time` and lets passengers
    /// off, and the day it runs on.
    fn find_latest_trip(
        &self,
        pattern: TripPatternIndex,
        position: PatternPosition,
        ready_time: Time,
    ) -> Option<(TripIndex, Day)> {
        self.runs(pattern)
            .filter_map(|(trip, day)| {
                let stop_time = self.stop_time_in(pattern, trip, position);
                if stop_time.drop_off == BoardingRule::NOT_AVAILABLE {
                    return None;
                }
                let arrival = self.query.arrival(&stop_time, day)?;
                Some((trip, day, arrival.actual))
            })
            .filter(|&(_, _, arrival)| arrival <= ready_time)
            .max_by_key(|&(_, _, arrival)| arrival)
            .map(|(trip, day, _)| (trip, day))
    }

    fn stop_time(&self, trip: TripIndex, position: PatternPosition) -> StopTime {
        let pattern = self.consumer.trip_pattern_by_trip(trip).index;
        self.stop_time_in(pattern, trip, position)
    }

    fn stop_time_in(
        &self,
        pattern: TripPatternIndex,
        trip: TripIndex,
        position: PatternPosition,
    ) -> StopTime {
        StopTime::new(self.consumer, pattern, trip, position.as_usize())
    }

    /// The arrival of a trip at a position, if the vehicle serves it.
    fn arrival(&self, trip: TripIndex, day: Day, position: PatternPosition) -> Option<LiveTime> {
        self.query.arrival(&self.stop_time(trip, position), day)
    }

    /// The departure of a trip from a position, if the vehicle serves it.
    fn departure(&self, trip: TripIndex, day: Day, position: PatternPosition) -> Option<LiveTime> {
        self.query.departure(&self.stop_time(trip, position), day)
    }

    fn transfers_forward(
        &self,
        state: &State,
        stop: StopIndex,
        label: ParetoLabel,
        buffer: &mut Vec<Update>,
    ) {
        for transfer in self.consumer.outbound_transfers_by_stop(stop) {
            let hop = Hop {
                from: stop,
                to: transfer.to_stop_index,
                seconds: self.transfer_seconds(
                    transfer.from_stop_index,
                    transfer.to_stop_index,
                    transfer.min_transfer_time,
                ),
                penalty: self.query.transfer_penalty,
                kind: StepKind::Transfer,
            };
            relax_forward(state, buffer, label, hop);
        }
        for hop in self.walks(stop) {
            relax_forward(state, buffer, label, hop);
        }
    }

    fn transfers_back(
        &self,
        state: &State,
        stop: StopIndex,
        label: ParetoLabel,
        buffer: &mut Vec<Update>,
    ) {
        for transfer in self.consumer.iter_inbound_transfers_by_stop(stop) {
            let hop = Hop {
                from: stop,
                to: transfer.from_stop_index,
                seconds: self.transfer_seconds(
                    transfer.from_stop_index,
                    transfer.to_stop_index,
                    transfer.min_transfer_time,
                ),
                penalty: self.query.transfer_penalty,
                kind: StepKind::Transfer,
            };
            relax_back(state, buffer, label, hop);
        }
        for hop in self.walks(stop) {
            relax_back(state, buffer, label, hop);
        }
    }

    /// The walks from a stop to the stops near it.
    fn walks(&self, stop: StopIndex) -> impl Iterator<Item = Hop> + 'a {
        let position = self.consumer.stop(stop).coordinate.as_option();
        let (consumer, spatial, radius) = (self.consumer, self.spatial, self.query.search_radius);
        let penalty = self.query.walk_penalty;
        position.into_iter().flat_map(move |position| {
            raptor::walkable_stops(consumer, spatial, position, radius).map(move |(to, walk)| Hop {
                from: stop,
                to,
                seconds: walk.0,
                penalty,
                kind: StepKind::Walk,
            })
        })
    }

    /// The time a transfer takes: the feed's minimum, or the walk between the stops when it gives
    /// none.
    fn transfer_seconds(&self, from: StopIndex, to: StopIndex, minimum: Optional<Duration>) -> u32 {
        let seconds = minimum.as_option().unwrap_or(Duration(0)).0;
        if seconds != 0 {
            return seconds;
        }
        let from = self.consumer.stop(from).coordinate.as_option();
        let to = self.consumer.stop(to).coordinate.as_option();
        from.zip(to)
            .map_or(0, |(from, to)| raptor::time_to_walk(from, to).0)
    }
}

/// Emits the label for arriving at `hop.to` after leaving `hop.from` at the label's time.
fn relax_forward(state: &State, buffer: &mut Vec<Update>, label: ParetoLabel, hop: Hop) {
    let arrival = label.time + Duration(hop.seconds);
    let cost = pareto::narrow_cost(f64::from(label.cost) + f64::from(hop.seconds) * hop.penalty);
    if state.is_dominated(hop.to, arrival, cost, false) {
        return;
    }
    buffer.push(Update::new(
        hop.to,
        arrival,
        cost,
        Parent::step(
            hop.kind,
            hop.from,
            LiveTime::scheduled_only(label.time),
            LiveTime::scheduled_only(arrival),
        ),
    ));
}

/// Emits the label for leaving `hop.to` in time to be at `hop.from` at the label's time.
fn relax_back(state: &State, buffer: &mut Vec<Update>, label: ParetoLabel, hop: Hop) {
    let Some(departure) = label.time.0.checked_sub(hop.seconds).map(Time) else {
        return;
    };
    let cost = pareto::narrow_cost(f64::from(label.cost) + f64::from(hop.seconds) * hop.penalty);
    if state.is_dominated(hop.to, departure, cost, true) {
        return;
    }
    buffer.push(Update::new(
        hop.to,
        departure,
        cost,
        Parent::step(
            hop.kind,
            hop.from,
            LiveTime::scheduled_only(departure),
            LiveTime::scheduled_only(label.time),
        ),
    ));
}

/// Joins two buffers by moving the shorter into the longer.
fn merge_buffers<T>(mut first: Vec<T>, mut second: Vec<T>) -> Vec<T> {
    if first.len() >= second.len() {
        first.append(&mut second);
        first
    } else {
        second.append(&mut first);
        second
    }
}
