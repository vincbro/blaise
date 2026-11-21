//! Plans every journey that leaves within a stretch of time.

mod common;

#[cfg(test)]
mod tests {
    use blaise_core::{
        Error,
        raptor::{itinerary::Itinerary, query::QueryLocation, state::State},
        spatial::SpatialGrid,
    };
    use flatnet_core::{consumer::Consumer, models::Duration};

    use crate::common::{self, query, stop, time};

    /// Trips from A to C that leave at 08:00, 08:10 and 08:20 and take 20 minutes, and a slow one
    /// that leaves at 08:05 and takes 35.
    fn every_ten_minutes() -> (tempfile::NamedTempFile, memmap2::Mmap) {
        common::network(
            "R,s,T1\nR,s,T2\nR,s,T3\nR,s,T4\n",
            "T1,08:00:00,08:00:00,A,1,0,0\nT1,08:20:00,08:20:00,C,2,0,0\n\
             T2,08:10:00,08:10:00,A,1,0,0\nT2,08:30:00,08:30:00,C,2,0,0\n\
             T3,08:20:00,08:20:00,A,1,0,0\nT3,08:40:00,08:40:00,C,2,0,0\n\
             T4,08:05:00,08:05:00,A,1,0,0\nT4,08:40:00,08:40:00,C,2,0,0\n",
        )
    }

    fn departures(
        consumer: &Consumer<'_>,
        start: &str,
        window: u32,
    ) -> blaise_core::Result<Vec<u32>> {
        let query = query(
            QueryLocation::Stop(stop(consumer, "A")),
            QueryLocation::Stop(stop(consumer, "C")),
            None,
            None,
        )
        .with_departure(time(start));
        let spatial = SpatialGrid::new(consumer);
        let mut state = State::new(consumer);

        let journeys = query.journeys_within(Duration(window), consumer, &spatial, &mut state)?;

        Ok(journeys
            .itineraries
            .iter()
            .map(|itinerary: &Itinerary| itinerary.legs[0].departure_time.actual.0)
            .collect())
    }

    #[test]
    fn every_journey_that_leaves_in_the_window_is_found_in_order() {
        let (_file, mmap) = every_ten_minutes();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let found = departures(&consumer, "08:00:00", 15 * 60).expect("journeys exist");

        assert_eq!(
            found,
            [time("08:00:00").0, time("08:10:00").0],
            "T1 and T2 leave in the window, T3 leaves after it"
        );
    }

    #[test]
    fn a_journey_that_leaves_earlier_and_arrives_later_is_left_out() {
        let (_file, mmap) = every_ten_minutes();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let found = departures(&consumer, "08:00:00", 30 * 60).expect("journeys exist");

        assert_eq!(found.len(), 3, "T4 leaves before T2 and arrives after it");
    }

    #[test]
    fn a_window_of_nothing_is_the_one_departure() {
        let (_file, mmap) = every_ten_minutes();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let found = departures(&consumer, "08:00:00", 0).expect("a journey exists");

        assert_eq!(found, [time("08:00:00").0], "only T1 leaves at 08:00");
    }

    #[test]
    fn a_window_with_no_departure_has_no_route() {
        let (_file, mmap) = every_ten_minutes();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let result = departures(&consumer, "09:00:00", 600);

        assert!(
            matches!(result, Err(Error::NoRoute)),
            "nothing leaves after 09:00"
        );
    }

    #[test]
    fn a_window_needs_a_departure_not_an_arrival() {
        let (_file, mmap) = every_ten_minutes();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let query = query(
            QueryLocation::Stop(stop(&consumer, "A")),
            QueryLocation::Stop(stop(&consumer, "C")),
            Some("09:00:00"),
            None,
        );
        let spatial = SpatialGrid::new(&consumer);
        let mut state = State::new(&consumer);

        let result = query.journeys_within(Duration(600), &consumer, &spatial, &mut state);

        assert!(
            matches!(result, Err(Error::WindowNeedsDeparture)),
            "a window counts forward from a departure"
        );
    }
}
