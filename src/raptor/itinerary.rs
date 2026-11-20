use std::ops::RangeInclusive;

use gtfs_bin::{
    consumer::Consumer,
    models::{Distance, Optional, StopIndex, StopTime, Time, TripIndex},
};

use crate::{
    Error, Result,
    raptor::{
        LiveTime,
        pareto::ParetoLabel,
        query::{Location, RaptorQuery, TimeDirection},
        state::State,
        update::Parent,
    },
};

#[derive(Debug, Clone)]
pub struct Leg {
    pub from: Location,
    pub to: Location,
    pub departure_time: LiveTime,
    pub arrival_time: LiveTime,
    pub stops: Box<[LegStop]>,
    pub leg_type: LegType,
}

#[derive(Debug, Clone, Copy)]
pub enum LegType {
    Transit(TripIndex),
    Transfer,
    Walk,
    Origin,
}

#[derive(Debug, Clone, Copy)]
pub struct LegStop {
    pub location: Location,
    pub departure_time: LiveTime,
    pub arrival_time: LiveTime,
    pub distance_traveled: Optional<Distance>,
}

impl LegStop {
    /// A stop of a transit leg. A stop the vehicle skips keeps its scheduled times.
    fn new(query: &RaptorQuery<'_>, stop: StopIndex, stop_time: &StopTime) -> Self {
        let scheduled =
            |time: Optional<Time>| LiveTime::scheduled_only(time.as_option().unwrap_or(Time(0)));
        Self {
            location: Location::Stop(stop),
            departure_time: query
                .departure(stop_time)
                .unwrap_or_else(|| scheduled(stop_time.departure_time)),
            arrival_time: query
                .arrival(stop_time)
                .unwrap_or_else(|| scheduled(stop_time.arrival_time)),
            distance_traveled: stop_time.distance_traveled,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Itinerary {
    pub from: Location,
    pub to: Location,
    pub legs: Box<[Leg]>,
}

/// Where a trace back through the parents has got to.
#[derive(Debug, Clone, Copy)]
struct Cursor {
    stop: StopIndex,
    round: usize,
    finished: bool,
}

/// What it takes to read an itinerary back out of a finished search.
struct Trace<'a> {
    query: &'a RaptorQuery<'a>,
    state: &'a State,
    consumer: &'a Consumer<'a>,
}

impl Itinerary {
    pub(crate) fn new(
        query: &RaptorQuery<'_>,
        state: &State,
        consumer: &Consumer<'_>,
    ) -> Result<Self> {
        let (best_stop, best_round) = state.best_target().ok_or(Error::NoRoute)?;
        let trace = Trace {
            query,
            state,
            consumer,
        };

        let mut legs = Vec::with_capacity((best_round * 2) + 2);
        legs.extend(trace.outer_walk(best_stop));
        let current_stop = trace.walk_back(best_stop, best_round, &mut legs);
        legs.extend(trace.inner_walk(current_stop));

        // A departure search is traced back from the destination, so its legs come out last-first.
        // An arrival search is traced from the origin, which is already the order of travel.
        if !query.is_arrival() {
            legs.reverse();
        }
        merge_walks(&mut legs);

        let (from, to) = if query.is_arrival() {
            (
                query.origin.resolve(best_stop),
                query.destination.resolve(current_stop),
            )
        } else {
            (
                query.origin.resolve(current_stop),
                query.destination.resolve(best_stop),
            )
        };
        Ok(Self {
            from,
            to,
            legs: legs.into_boxed_slice(),
        })
    }
}

impl Trace<'_> {
    /// The walk between the stop the search ended at and the far end of the journey, if they differ.
    fn outer_walk(&self, best_stop: StopIndex) -> Option<Leg> {
        let (from, to) = match self.query.time_direction {
            TimeDirection::Departure(_) => (
                Location::Stop(best_stop),
                self.query.destination.resolve(best_stop),
            ),
            TimeDirection::Arrival(_) => (
                self.query.origin.resolve(best_stop),
                Location::Stop(best_stop),
            ),
        };
        if from == to {
            return None;
        }

        let stop_label = self
            .state
            .cheapest_label(best_stop)
            .unwrap_or(ParetoLabel::UNREACHED);
        let target_label = self.state.cheapest_target_label().unwrap_or(stop_label);
        let (departure, arrival) = match self.query.time_direction {
            TimeDirection::Departure(_) => (stop_label, target_label),
            TimeDirection::Arrival(_) => (target_label, stop_label),
        };
        Some(walk_leg(from, to, departure.time, arrival.time))
    }

    /// The walk between the journey's near end and the first stop the search left, if they differ.
    fn inner_walk(&self, current_stop: StopIndex) -> Option<Leg> {
        let label = self
            .state
            .cheapest_label(current_stop)
            .unwrap_or(ParetoLabel::UNREACHED);
        let (from, to, departure, arrival) = match self.query.time_direction {
            TimeDirection::Departure(departure) => (
                self.query.origin.resolve(current_stop),
                Location::Stop(current_stop),
                departure,
                label.time,
            ),
            TimeDirection::Arrival(arrival) => (
                Location::Stop(current_stop),
                self.query.destination.resolve(current_stop),
                label.time,
                arrival,
            ),
        };
        (from != to).then(|| walk_leg(from, to, departure, arrival))
    }

    /// Follows the parents from the stop the search ended at back to where it began, adding a leg
    /// for each, and returns the stop it reached.
    fn walk_back(&self, best_stop: StopIndex, best_round: usize, legs: &mut Vec<Leg>) -> StopIndex {
        let mut cursor = Cursor {
            stop: best_stop,
            round: best_round,
            finished: false,
        };
        // A stop reachable straight from the origin needs no walks or transfers before it.
        while !cursor.finished && !self.is_reachable_from_origin(cursor.stop) {
            self.follow_parent(&mut cursor, legs);
        }
        cursor.stop
    }

    fn is_reachable_from_origin(&self, stop: StopIndex) -> bool {
        matches!(self.state.parent(0, stop), Some(Parent::Origin))
    }

    /// Adds the leg that led to the cursor's stop and moves the cursor to where the leg started.
    fn follow_parent(&self, cursor: &mut Cursor, legs: &mut Vec<Leg>) {
        let parent = self
            .state
            .parent(cursor.round, cursor.stop)
            .expect("every label on the path was found with a parent");
        match parent {
            Parent::Transit {
                boarding_position,
                alighting_position,
                trip,
                departure_time,
                arrival_time,
            } => {
                let positions = boarding_position.as_usize()..=alighting_position.as_usize();
                self.follow_transit(
                    cursor,
                    legs,
                    trip,
                    positions,
                    [departure_time, arrival_time],
                );
            }
            Parent::Transfer {
                from_stop,
                departure_time,
                arrival_time,
            }
            | Parent::Walk {
                from_stop,
                departure_time,
                arrival_time,
            } => {
                let leg_type = if matches!(parent, Parent::Transfer { .. }) {
                    LegType::Transfer
                } else {
                    LegType::Walk
                };
                legs.push(self.step_leg(
                    cursor.stop,
                    from_stop,
                    [departure_time, arrival_time],
                    leg_type,
                ));
                cursor.stop = from_stop;
            }
            Parent::Origin => cursor.finished = true,
        }
    }

    /// Adds a ride, and moves the cursor to the stop it was boarded at going forward, or left at
    /// going back, and one round earlier.
    fn follow_transit(
        &self,
        cursor: &mut Cursor,
        legs: &mut Vec<Leg>,
        trip: TripIndex,
        positions: RangeInclusive<usize>,
        times: [LiveTime; 2],
    ) {
        let pattern = self.consumer.trip_pattern_by_trip(trip);
        let stops = self.consumer.stop_sequence_by_trip_pattern(pattern.index);
        let (boarding, alighting) = (stops[*positions.start()], stops[*positions.end()]);

        let stop_times = self.consumer.stop_times_by_trip(trip);
        let leg_stops = positions
            .map(|position| LegStop::new(self.query, stops[position], &stop_times[position]))
            .collect();
        legs.push(Leg {
            from: Location::Stop(boarding),
            to: Location::Stop(alighting),
            departure_time: times[0],
            arrival_time: times[1],
            stops: leg_stops,
            leg_type: LegType::Transit(trip),
        });

        cursor.stop = if self.query.is_arrival() {
            alighting
        } else {
            boarding
        };
        match cursor.round.checked_sub(1) {
            Some(round) => cursor.round = round,
            None => cursor.finished = true,
        }
    }

    /// A transfer or a walk between `stop` and `from_stop`, in the direction the search ran.
    fn step_leg(
        &self,
        stop: StopIndex,
        from_stop: StopIndex,
        times: [LiveTime; 2],
        leg_type: LegType,
    ) -> Leg {
        let (from, to) = if self.query.is_arrival() {
            (stop, from_stop)
        } else {
            (from_stop, stop)
        };
        Leg {
            from: Location::Stop(from),
            to: Location::Stop(to),
            departure_time: times[0],
            arrival_time: times[1],
            stops: Box::default(),
            leg_type,
        }
    }
}

fn walk_leg(from: Location, to: Location, departure: Time, arrival: Time) -> Leg {
    Leg {
        from,
        to,
        departure_time: LiveTime::scheduled_only(departure),
        arrival_time: LiveTime::scheduled_only(arrival),
        stops: Box::default(),
        leg_type: LegType::Walk,
    }
}

/// Joins walks that follow each other into one.
fn merge_walks(legs: &mut Vec<Leg>) {
    if legs.is_empty() {
        return;
    }
    let mut write = 0;
    for read in 1..legs.len() {
        let both_walks = matches!(legs[write].leg_type, LegType::Walk)
            && matches!(legs[read].leg_type, LegType::Walk);
        if both_walks {
            legs[write].to = legs[read].to;
            legs[write].arrival_time = legs[read].arrival_time;
        } else {
            write += 1;
            legs.swap(write, read);
        }
    }
    legs.truncate(write + 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leg(leg_type: LegType, from: u32, to: u32, departure: u32, arrival: u32) -> Leg {
        Leg {
            from: Location::Stop(StopIndex(from)),
            to: Location::Stop(StopIndex(to)),
            departure_time: LiveTime::scheduled_only(Time(departure)),
            arrival_time: LiveTime::scheduled_only(Time(arrival)),
            stops: Box::default(),
            leg_type,
        }
    }

    #[test]
    fn walks_that_follow_each_other_become_one() {
        let mut legs = vec![
            leg(LegType::Walk, 0, 1, 0, 10),
            leg(LegType::Walk, 1, 2, 10, 25),
            leg(LegType::Transit(TripIndex(0)), 2, 3, 30, 60),
            leg(LegType::Walk, 3, 4, 60, 70),
        ];

        merge_walks(&mut legs);

        assert_eq!(legs.len(), 3, "the two first walks are one");
        assert_eq!(
            legs[0].to,
            Location::Stop(StopIndex(2)),
            "it ends where the second ended"
        );
        assert_eq!(
            legs[0].arrival_time.actual,
            Time(25),
            "and arrives when it did"
        );
        assert!(
            matches!(legs[1].leg_type, LegType::Transit(_)),
            "the ride is kept"
        );
    }

    #[test]
    fn a_walk_and_a_transfer_are_not_merged() {
        let mut legs = vec![
            leg(LegType::Walk, 0, 1, 0, 10),
            leg(LegType::Transfer, 1, 2, 10, 20),
        ];

        merge_walks(&mut legs);

        assert_eq!(legs.len(), 2, "only walks merge");
    }

    #[test]
    fn no_legs_stay_no_legs() {
        let mut legs: Vec<Leg> = Vec::new();

        merge_walks(&mut legs);

        assert!(legs.is_empty(), "nothing to merge");
    }
}
