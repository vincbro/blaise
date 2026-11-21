use flatnet_core::{
    consumer::Consumer,
    models::{
        BoardingRule, Coordinate, Date, Distance, Optional, StopIndex, StopTimeIndex, Time, Timing,
        TripIndex, TripPatternIndex,
    },
    rt::Realtime,
};
use jiff::ZonedDateTime;

use crate::{
    Result,
    raptor::{self, Day, LiveTime, itinerary::Itinerary, state::State},
    spatial::SpatialGrid,
};

/// A trip's visit to one stop of its pattern: its times, what the stop allows, and where the
/// visit sits in the file, which is what a [`Realtime`] is indexed by.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StopTime {
    pub(crate) trip: TripIndex,
    pub(crate) index: StopTimeIndex,
    pub(crate) timing: Timing,
    pub(crate) pickup: BoardingRule,
    pub(crate) drop_off: BoardingRule,
    pub(crate) distance_traveled: Optional<Distance>,
}

impl StopTime {
    /// The visit of `trip`, which runs `pattern`, at `position` in that pattern.
    pub(crate) fn new(
        consumer: &Consumer<'_>,
        pattern: TripPatternIndex,
        trip: TripIndex,
        position: usize,
    ) -> Self {
        let first = consumer.trip(trip).stop_times.start as usize;
        let rule = &consumer.pattern_stops_by_trip_pattern(pattern)[position];
        Self {
            trip,
            index: StopTimeIndex::from(first + position),
            timing: consumer.timings_by_trip(trip)[position],
            pickup: rule.pickup,
            drop_off: rule.drop_off,
            distance_traveled: rule.distance_traveled,
        }
    }
}

/// Where a journey starts or ends, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryLocation {
    Stop(StopIndex),
    Stops(Box<[StopIndex]>),
    Coordinate(Coordinate),
}

impl From<Coordinate> for QueryLocation {
    fn from(value: Coordinate) -> Self {
        Self::Coordinate(value)
    }
}

impl From<StopIndex> for QueryLocation {
    fn from(value: StopIndex) -> Self {
        Self::Stop(value)
    }
}

impl From<Vec<StopIndex>> for QueryLocation {
    fn from(value: Vec<StopIndex>) -> Self {
        Self::Stops(value.into_boxed_slice())
    }
}

impl QueryLocation {
    /// The place an itinerary names for this end of the journey when it uses `stop`.
    #[must_use]
    pub(crate) const fn resolve(&self, stop: StopIndex) -> Location {
        match self {
            Self::Coordinate(coordinate) => Location::Coordinate(*coordinate),
            _ => Location::Stop(stop),
        }
    }
}

/// A place in an itinerary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    Stop(StopIndex),
    Coordinate(Coordinate),
}

impl From<Coordinate> for Location {
    fn from(value: Coordinate) -> Self {
        Self::Coordinate(value)
    }
}

impl From<StopIndex> for Location {
    fn from(value: StopIndex) -> Self {
        Self::Stop(value)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TimeDirection {
    Arrival(Time),
    Departure(Time),
}

impl TimeDirection {
    /// The time the journey leaves or arrives at.
    #[must_use]
    pub const fn time(self) -> Time {
        match self {
            Self::Arrival(time) | Self::Departure(time) => time,
        }
    }

    /// The same direction with its time moved from the query's date to the search clock, which
    /// starts a day earlier.
    #[must_use]
    pub(crate) fn on_search_clock(self) -> Self {
        match self {
            Self::Arrival(time) => Self::Arrival(Time(time.0.saturating_add(Day::QUERY.shift()))),
            Self::Departure(time) => {
                Self::Departure(Time(time.0.saturating_add(Day::QUERY.shift())))
            }
        }
    }
}

/// A journey to plan.
#[derive(Debug)]
pub struct RaptorQuery<'a> {
    pub origin: QueryLocation,
    pub destination: QueryLocation,
    pub time_direction: TimeDirection,
    pub date: Date,
    /// How far in meters to look for stops to walk to.
    pub search_radius: f64,
    /// Live delays, cancellations and skipped stops. Without it the query uses the timetable.
    pub realtime: Option<&'a Realtime>,

    pub transit_penalty: f64,
    pub transfer_penalty: f64,
    pub walk_penalty: f64,
    /// A fixed cost, in weighted seconds, for every vehicle boarded. It makes a journey with
    /// fewer vehicles beat one that is a little faster but changes more, whether the change is at
    /// the same stop or not. `0.0` ranks journeys by their weighted time alone.
    pub boarding_penalty: f64,
}

impl<'a> RaptorQuery<'a> {
    /// A journey that leaves now, today.
    #[must_use]
    pub fn new(origin: QueryLocation, destination: QueryLocation) -> Self {
        // The local day and time of day, as the agency counts them when it is in this zone.
        let now = ZonedDateTime::now();
        let seconds =
            i64::from(now.hour()) * 3600 + i64::from(now.minute()) * 60 + i64::from(now.second());
        let date = Date::from_year_month_day(
            u32::try_from(now.year()).unwrap_or(1970),
            u32::try_from(now.month()).unwrap_or(1),
            u32::try_from(now.day()).unwrap_or(1),
        )
        .unwrap_or_default();

        Self {
            origin,
            destination,
            time_direction: TimeDirection::Departure(Time::from(seconds)),
            search_radius: 1000.0,
            date,
            realtime: None,

            transit_penalty: 1.0,
            transfer_penalty: 1.5,
            walk_penalty: 2.0,
            boarding_penalty: 180.0,
        }
    }

    #[must_use]
    pub const fn with_arrival(mut self, arrival: Time) -> Self {
        self.time_direction = TimeDirection::Arrival(arrival);
        self
    }

    #[must_use]
    pub const fn with_departure(mut self, departure: Time) -> Self {
        self.time_direction = TimeDirection::Departure(departure);
        self
    }

    #[must_use]
    pub const fn with_date(mut self, date: Date) -> Self {
        self.date = date;
        self
    }

    /// Sets the fixed cost of boarding a vehicle, see [`Self::boarding_penalty`].
    #[must_use]
    pub const fn with_boarding_penalty(mut self, penalty: f64) -> Self {
        self.boarding_penalty = penalty;
        self
    }

    /// Plans on live times. A `Realtime` made for a service date applies to the trips of that
    /// date, which can be the day before the query's, its own or the day after. One made without
    /// a date applies to the query's.
    #[must_use]
    pub const fn with_realtime(mut self, realtime: &'a Realtime) -> Self {
        self.realtime = Some(realtime);
        self
    }

    /// Whether the journey is planned back from the time it must arrive by.
    #[must_use]
    pub const fn is_arrival(&self) -> bool {
        matches!(self.time_direction, TimeDirection::Arrival(_))
    }

    /// The service date a trip on `day` runs on, or `None` before the first date.
    #[must_use]
    pub(crate) fn service_date(&self, day: Day) -> Option<Date> {
        let day_of_query = u32::from(day.offset());
        self.date
            .0
            .checked_add(day_of_query)?
            .checked_sub(1)
            .map(Date)
    }

    /// The arrival at a stop time of a trip on `day`, scheduled and expected, on the search
    /// clock, or `None` if the vehicle skips the stop, the trip is canceled, or the stop has no
    /// arrival.
    #[must_use]
    pub(crate) fn arrival(&self, stop_time: &StopTime, day: Day) -> Option<LiveTime> {
        let scheduled = stop_time.timing.arrival.as_option()?;
        let actual = match self.live(day) {
            Some(realtime) => {
                realtime.arrival_time(stop_time.trip, stop_time.index, stop_time.timing)?
            }
            None => scheduled,
        };
        Some(LiveTime::new(scheduled, actual).shifted(day.shift()))
    }

    /// The departure from a stop time of a trip on `day`, like [`Self::arrival`].
    #[must_use]
    pub(crate) fn departure(&self, stop_time: &StopTime, day: Day) -> Option<LiveTime> {
        let scheduled = stop_time.timing.departure.as_option()?;
        let actual = match self.live(day) {
            Some(realtime) => {
                realtime.departure_time(stop_time.trip, stop_time.index, stop_time.timing)?
            }
            None => scheduled,
        };
        Some(LiveTime::new(scheduled, actual).shifted(day.shift()))
    }

    /// Plans the journey.
    ///
    /// # Errors
    ///
    /// Returns an error if no route reaches the destination.
    pub fn solve(
        self,
        consumer: &Consumer<'_>,
        spatial: &SpatialGrid,
        state: &mut State,
    ) -> Result<Itinerary> {
        raptor::solve(self, consumer, spatial, state)
    }

    /// The live data for trips on `day`. A `Realtime` made for a service date describes that date
    /// only, and one made without a date describes the query's.
    fn live(&self, day: Day) -> Option<&'a Realtime> {
        self.realtime
            .filter(|realtime| match realtime.service_date() {
                Some(date) => self.service_date(day) == Some(date),
                None => day == Day::QUERY,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;

    #[test]
    fn the_time_of_a_direction_is_the_time_it_holds() {
        assert_eq!(TimeDirection::Arrival(Time(7)).time(), Time(7), "arrival");
        assert_eq!(
            TimeDirection::Departure(Time(9)).time(),
            Time(9),
            "departure"
        );
    }

    #[test]
    fn a_query_is_for_an_arrival_only_after_with_arrival() {
        let query = RaptorQuery::new(StopIndex(0).into(), StopIndex(1).into());

        assert!(!query.is_arrival(), "a new query leaves now");
        assert!(
            query.with_arrival(Time(100)).is_arrival(),
            "with_arrival plans back"
        );
    }

    #[test]
    fn a_coordinate_keeps_its_place_in_the_itinerary_and_a_stop_set_becomes_the_stop_used() {
        let coordinate = Coordinate::new(59.0, 18.0);

        assert_eq!(
            QueryLocation::Coordinate(coordinate).resolve(StopIndex(3)),
            Location::Coordinate(coordinate),
            "the passenger names a place, not the stop used"
        );
        assert_eq!(
            QueryLocation::Stops(vec![StopIndex(1), StopIndex(2)].into()).resolve(StopIndex(2)),
            Location::Stop(StopIndex(2)),
            "a set of stops resolves to the one the route uses"
        );
    }

    #[test]
    fn a_stop_time_has_the_timing_and_position_of_its_visit() {
        let network = test_support::network(&[(59.0, 18.0), (59.1, 18.0)], &[vec![0, 1]]);
        let consumer = network.consumer();
        let trip = TripIndex(0);
        let pattern = consumer.trip_pattern_by_trip(trip).index;

        let stop_time = StopTime::new(&consumer, pattern, trip, 1);

        assert_eq!(
            stop_time.index,
            StopTimeIndex::from(consumer.trip(trip).stop_times.start as usize + 1),
            "the second visit follows the first in the file"
        );
        assert_eq!(
            stop_time.timing,
            consumer.timings_by_trip(trip)[1],
            "the timing is the trip's second"
        );
        assert_eq!(
            stop_time.drop_off,
            BoardingRule::default(),
            "the test feed lets passengers off"
        );
    }
}
