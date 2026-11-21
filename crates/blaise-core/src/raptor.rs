use flatnet_core::{
    consumer::Consumer,
    models::{Coordinate, Delay, Duration, Sentinel, StopIndex, Time, TripPatternIndex},
};

use crate::{
    Error, Result,
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
pub mod journeys;
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

/// Seconds in a service day.
pub(crate) const DAY_SECONDS: u32 = 86_400;

/// Which service day a trip runs on, counted from the day before the query's date.
///
/// A search keeps time from midnight of the day before the query's date, so a trip of yesterday's
/// service that runs past midnight, and one of tomorrow's service, are both on the same clock as
/// the query. The clock only exists inside a search: an itinerary is shifted back to the date it
/// starts on.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Day(u8);

impl Day {
    /// The day before the query's date, the query's date and the day after it.
    pub(crate) const ALL: [Self; 3] = [Self(0), Self(1), Self(2)];
    /// The query's own date.
    pub(crate) const QUERY: Self = Self(1);

    /// The number of days after the first day of the search clock.
    pub(crate) const fn offset(self) -> u8 {
        self.0
    }

    /// How far the stop times of a trip on this day move on the search clock.
    pub(crate) fn shift(self) -> u32 {
        u32::from(self.0) * DAY_SECONDS
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

    /// A time from the timetable and the time the vehicle is expected.
    #[must_use]
    pub(crate) const fn new(scheduled: Time, actual: Time) -> Self {
        Self { scheduled, actual }
    }

    /// The same times, `seconds` later.
    #[must_use]
    pub(crate) fn shifted(self, seconds: u32) -> Self {
        Self {
            scheduled: Time(self.scheduled.0.saturating_add(seconds)),
            actual: Time(self.actual.0.saturating_add(seconds)),
        }
    }

    /// The same times, `seconds` earlier, and not before the start of the clock.
    #[must_use]
    pub(crate) const fn earlier(self, seconds: u32) -> Self {
        Self {
            scheduled: Time(self.scheduled.0.saturating_sub(seconds)),
            actual: Time(self.actual.0.saturating_sub(seconds)),
        }
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
/// The search runs on a clock that starts a day before the query's date. The
/// itinerary is shifted back to the day its first leg starts on.
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
    let query = RaptorQuery {
        time_direction: query.time_direction.on_search_clock(),
        ..query
    };
    let itinerary = solve_core(&query, consumer, spatial, state)?;

    if query.is_arrival()
        && let Some(first_leg) = itinerary.legs.first()
    {
        let forward_query = RaptorQuery {
            time_direction: TimeDirection::Departure(first_leg.departure_time.actual),
            ..query
        };
        state.reset();
        return solve_core(&forward_query, consumer, spatial, state).map(Itinerary::rebased);
    }

    Ok(itinerary.rebased())
}

/// Every journey that gets there sooner than any with fewer vehicles, fewest vehicles first.
pub fn solve_all(
    query: RaptorQuery<'_>,
    consumer: &Consumer<'_>,
    spatial: &SpatialGrid,
    state: &mut State,
) -> Result<Vec<Itinerary>> {
    let query = RaptorQuery {
        time_direction: query.time_direction.on_search_clock(),
        ..query
    };
    run_rounds(&query, consumer, spatial, state);
    let targets = state.round_targets(query.is_arrival());
    if targets.is_empty() {
        return Err(Error::NoRoute);
    }
    if !query.is_arrival() {
        return Ok(targets
            .into_iter()
            .map(|(stop, round)| Itinerary::at(&query, state, consumer, stop, round).rebased())
            .collect());
    }

    // Each journey found going back leaves at its own time, so it is planned forward from there.
    // The ways forward that still get there in time are all kept, because the latest departure
    // says nothing about which of them arrives first.
    let deadline = query.time_direction.time();
    let departures: Vec<Time> = targets
        .into_iter()
        .filter_map(|(stop, round)| {
            let itinerary = Itinerary::at(&query, state, consumer, stop, round);
            Some(itinerary.legs.first()?.departure_time.actual)
        })
        .collect();
    let mut found: Vec<Itinerary> = Vec::with_capacity(departures.len());
    for departure in departures {
        let forward_query = query.leaving_at(departure);
        state.reset();
        run_rounds(&forward_query, consumer, spatial, state);
        for (stop, round) in state.round_targets(false) {
            let itinerary = Itinerary::at(&forward_query, state, consumer, stop, round);
            let in_time = itinerary
                .arrival()
                .is_some_and(|arrival| arrival.actual <= deadline);
            let itinerary = itinerary.rebased();
            if in_time && !found.iter().any(|kept| kept.same_journey(&itinerary)) {
                found.push(itinerary);
            }
        }
    }
    if found.is_empty() {
        return Err(Error::NoRoute);
    }
    Ok(found)
}

/// Up to `count` journeys, the best first, each found without the main vehicle of those before.
pub fn solve_alternatives(
    query: &RaptorQuery<'_>,
    count: usize,
    consumer: &Consumer<'_>,
    spatial: &SpatialGrid,
    state: &mut State,
) -> Result<Vec<Itinerary>> {
    let mut banned: Vec<TripPatternIndex> = Vec::new();
    let mut found: Vec<Itinerary> = Vec::with_capacity(count);
    while found.len() < count {
        state.reset();
        let Ok(itinerary) = solve(query.avoiding(&banned), consumer, spatial, state) else {
            break;
        };
        let main = itinerary.main_pattern(consumer);
        found.push(itinerary);
        // A journey with no vehicle has nothing to avoid, so asking again would repeat it.
        let Some(main) = main else {
            break;
        };
        banned.push(main);
    }
    if found.is_empty() {
        return Err(Error::NoRoute);
    }
    Ok(found)
}

/// The journeys that leave from the query's departure time to `window` after it, without those
/// another beats by leaving later, arriving sooner and boarding no more vehicles.
pub fn solve_window(
    query: &RaptorQuery<'_>,
    window: Duration,
    consumer: &Consumer<'_>,
    spatial: &SpatialGrid,
    state: &mut State,
) -> Result<Vec<Itinerary>> {
    let TimeDirection::Departure(start) = query.time_direction else {
        return Err(Error::WindowNeedsDeparture);
    };
    let end = start.0.saturating_add(window.0);
    let mut found: Vec<(Itinerary, [u32; 2])> = Vec::new();
    let mut from = start;
    while from.0 <= end {
        state.reset();
        let Ok(itineraries) = solve_all(query.leaving_at(from), consumer, spatial, state) else {
            break;
        };
        let mut next = u32::MAX;
        for itinerary in itineraries {
            let (Some(leave), Some(arrive)) = (
                itinerary.leave_by(query.date),
                itinerary.arrival_from(query.date),
            ) else {
                continue;
            };
            next = next.min(leave.saturating_add(1));
            if leave <= end {
                found.push((itinerary, [leave, arrive]));
            }
        }
        // A journey with no vehicle leaves when asked, so a later start finds nothing new.
        from = Time(next.max(from.0.saturating_add(1)));
    }
    if found.is_empty() {
        return Err(Error::NoRoute);
    }
    Ok(unbeaten(found))
}

/// The journeys of `found` that no other beats, in order of leaving. Each has when the rider must
/// leave and when it arrives.
fn unbeaten(mut found: Vec<(Itinerary, [u32; 2])>) -> Vec<Itinerary> {
    let beats = |other: &(Itinerary, [u32; 2]), this: &(Itinerary, [u32; 2])| {
        let (leave, arrive, rides) = (other.1[0], other.1[1], other.0.rides());
        let no_worse = leave >= this.1[0] && arrive <= this.1[1] && rides <= this.0.rides();
        let same = leave == this.1[0] && arrive == this.1[1] && rides == this.0.rides();
        no_worse && !same
    };
    let mut kept: Vec<(Itinerary, [u32; 2])> = Vec::new();
    for candidate in found.drain(..) {
        let duplicate = kept
            .iter()
            .any(|other| other.1 == candidate.1 && other.0.rides() == candidate.0.rides());
        if !duplicate {
            kept.push(candidate);
        }
    }
    let all: Vec<bool> = kept
        .iter()
        .map(|this| !kept.iter().any(|other| beats(other, this)))
        .collect();
    let mut result: Vec<(Itinerary, [u32; 2])> = kept
        .into_iter()
        .zip(all)
        .filter_map(|(entry, unbeaten)| unbeaten.then_some(entry))
        .collect();
    result.sort_by_key(|(itinerary, [leave, _])| (*leave, itinerary.rides()));
    result.into_iter().map(|(itinerary, _)| itinerary).collect()
}

fn solve_core(
    query: &RaptorQuery<'_>,
    consumer: &Consumer<'_>,
    spatial: &SpatialGrid,
    state: &mut State,
) -> Result<Itinerary> {
    run_rounds(query, consumer, spatial, state);
    Itinerary::new(query, state, consumer)
}

/// Runs the rounds of the search, leaving the labels and parents in `state`.
fn run_rounds(
    query: &RaptorQuery<'_>,
    consumer: &Consumer<'_>,
    spatial: &SpatialGrid,
    state: &mut State,
) {
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
            .filter(|&&stop| !consumer.trip_patterns_by_stop(stop).is_empty())
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
        .filter(|&stop| !consumer.trip_patterns_by_stop(stop).is_empty())
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
