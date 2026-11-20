use gtfs_bin::{
    consumer::Consumer,
    models::{Coordinate, Delay, Duration, Sentinel, StopIndex, Time},
};

use crate::{
    Result,
    raptor::{
        explorer::Explorer,
        itinerary::Itinerary,
        pareto::ParetoLabel,
        query::{QueryLocation, RaptorQuery, TimeDirection},
        state::State,
    },
    spatial::SpatialGrid,
};

mod explorer;
pub mod itinerary;
mod pareto;
pub mod query;
pub mod state;
mod update;

const MAX_ROUNDS: usize = 15;

/// How fast a passenger walks, in meters per second.
const WALKING_SPEED: f64 = 1.2;
/// How much longer a walk is than the straight line between its ends.
const DETOUR_FACTOR: f64 = 1.3;

/// A position in a trip pattern's list of stops.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct PatternPosition(u32);

impl Sentinel for PatternPosition {
    const NONE: Self = Self(u32::MAX);
}

impl From<usize> for PatternPosition {
    fn from(value: usize) -> Self {
        u32::try_from(value).map_or(Self::NONE, Self)
    }
}

impl PatternPosition {
    #[must_use]
    pub(crate) const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

/// A time from the timetable and the time the vehicle is expected, which differ when it runs
/// early or late.
#[derive(Debug, Clone, Copy)]
pub struct LiveTime {
    pub scheduled: Time,
    pub actual: Time,
}

impl LiveTime {
    #[must_use]
    pub fn delay(&self) -> Delay {
        Delay::from(i64::from(self.actual.0) - i64::from(self.scheduled.0))
    }

    #[must_use]
    pub const fn scheduled_only(time: Time) -> Self {
        Self {
            scheduled: time,
            actual: time,
        }
    }
}

/// Finds the best itinerary for a query.
///
/// An arrive-by query is solved backwards to find the latest departure that still arrives in
/// time, then forwards from that departure so the itinerary is the usual one.
///
/// # Errors
///
/// Returns an error if no route reaches the destination.
pub fn solve(
    query: RaptorQuery<'_>,
    consumer: &Consumer<'_>,
    spatial: &SpatialGrid,
    state: &mut State,
) -> Result<Itinerary> {
    let itinerary = solve_core(&query, consumer, spatial, state)?;

    if query.is_arrival()
        && let Some(first_leg) = itinerary.legs.first()
    {
        let forward_query = RaptorQuery {
            time_direction: TimeDirection::Departure(first_leg.departure_time.actual),
            ..query
        };
        state.reset();
        return solve_core(&forward_query, consumer, spatial, state);
    }

    Ok(itinerary)
}

fn solve_core(
    query: &RaptorQuery<'_>,
    consumer: &Consumer<'_>,
    spatial: &SpatialGrid,
    state: &mut State,
) -> Result<Itinerary> {
    let is_arrival = query.is_arrival();
    let explorer = Explorer::new(query, consumer, spatial);

    seed(query, consumer, spatial, state);
    let updates = explorer.transfers(state);
    state.apply_updates(0, is_arrival, &updates);
    state.record_targets(0, is_arrival, query.walk_penalty);

    for round in 1..MAX_ROUNDS {
        if !state.has_marked_stops() {
            break;
        }
        state.begin_round(consumer, is_arrival);

        let updates = explorer.trip_patterns(state);
        state.apply_updates(round, is_arrival, &updates);
        let updates = explorer.transfers(state);
        state.apply_updates(round, is_arrival, &updates);
        state.record_targets(round, is_arrival, query.walk_penalty);
    }

    Itinerary::new(query, state, consumer)
}

/// Starts the search at the query's origin, or at its destination when searching back from an
/// arrival, and records the stops the other end can be reached from.
fn seed(
    query: &RaptorQuery<'_>,
    consumer: &Consumer<'_>,
    spatial: &SpatialGrid,
    state: &mut State,
) {
    let is_arrival = query.is_arrival();
    let (start, goal) = if is_arrival {
        (&query.destination, &query.origin)
    } else {
        (&query.origin, &query.destination)
    };
    let time = query.time_direction.time();

    for_each_endpoint(
        start,
        consumer,
        spatial,
        query.search_radius,
        |stop, walk| {
            let label_time = if is_arrival { time - walk } else { time + walk };
            let cost = pareto::narrow_cost(f64::from(walk.0) * query.walk_penalty);
            state.seed(stop, ParetoLabel::new(label_time, cost), is_arrival);
        },
    );
    for_each_endpoint(
        goal,
        consumer,
        spatial,
        query.search_radius,
        |stop, walk| {
            state.add_target(stop, walk);
        },
    );
}

/// Calls `visit` with each stop a place can be left from or reached at, and how long the walk
/// between the place and the stop takes.
fn for_each_endpoint(
    location: &QueryLocation,
    consumer: &Consumer<'_>,
    spatial: &SpatialGrid,
    search_radius: f64,
    mut visit: impl FnMut(StopIndex, Duration),
) {
    match location {
        QueryLocation::Stop(stop) => visit(*stop, Duration(0)),
        QueryLocation::Stops(stops) => stops
            .iter()
            .filter(|&&stop| !consumer.trips_by_stop(stop).is_empty())
            .for_each(|&stop| visit(stop, Duration(0))),
        QueryLocation::Coordinate(coordinate) => {
            walkable_stops(consumer, spatial, *coordinate, search_radius)
                .for_each(|(stop, walk)| visit(stop, walk));
        }
    }
}

/// The stops with a position and at least one trip within `radius` meters of a coordinate, with
/// the time it takes to walk to each.
pub(crate) fn walkable_stops<'a>(
    consumer: &'a Consumer<'_>,
    spatial: &'a SpatialGrid,
    coordinate: Coordinate,
    radius: f64,
) -> impl Iterator<Item = (StopIndex, Duration)> + 'a {
    spatial
        .iter_stops_in_radius(coordinate, radius)
        .filter(|&stop| !consumer.trips_by_stop(stop).is_empty())
        .filter_map(move |stop| {
            let position = consumer.stop(stop).coordinate.as_option()?;
            Some((stop, time_to_walk(coordinate, position)))
        })
}

/// How long it takes to walk between two coordinates, along streets rather than the straight
/// line.
#[must_use]
pub(crate) fn time_to_walk(from: Coordinate, to: Coordinate) -> Duration {
    let distance = from.distance_meters(to) * DETOUR_FACTOR;
    Duration(whole_seconds(distance / WALKING_SPEED))
}

/// Rounds a duration in seconds up to a whole second.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a walk is never negative, and a day of walking fits in u32"
)]
fn whole_seconds(seconds: f64) -> u32 {
    seconds.ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_partial_second_of_walking_counts_as_a_whole_one() {
        assert_eq!(whole_seconds(0.0), 0, "no walk takes no time");
        assert_eq!(whole_seconds(10.01), 11, "rounds up");
        assert_eq!(whole_seconds(10.0), 10, "a whole second stays");
    }

    #[test]
    fn walking_a_degree_of_latitude_takes_the_detour_and_the_speed_into_account() {
        let from = Coordinate::new(59.0, 18.0);
        let to = Coordinate::new(60.0, 18.0);

        let seconds = time_to_walk(from, to).0;

        // 111.2 km, 1.3 times as far along streets, at 1.2 meters per second.
        assert!((120_300..120_600).contains(&seconds), "{seconds} s");
        assert_eq!(
            time_to_walk(from, from),
            Duration(0),
            "standing still takes no time"
        );
    }

    #[test]
    fn a_delay_is_the_expected_time_less_the_scheduled_one() {
        let late = LiveTime {
            scheduled: Time(100),
            actual: Time(160),
        };
        let early = LiveTime {
            scheduled: Time(100),
            actual: Time(90),
        };

        assert_eq!(late.delay(), Delay(60), "a minute late");
        assert_eq!(early.delay(), Delay(-10), "ten seconds early");
        assert_eq!(
            LiveTime::scheduled_only(Time(5)).delay(),
            Delay(0),
            "on time"
        );
    }
}
