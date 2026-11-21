//! Plans journeys where changing vehicles could be avoided.

mod common;

#[cfg(test)]
mod tests {
    use blaise_core::{
        raptor::{itinerary::Itinerary, journeys::Journeys, query::QueryLocation, state::State},
        spatial::SpatialGrid,
    };
    use flatnet_core::{consumer::Consumer, models::Duration};

    use crate::common::{self, query, rides, solve, stop};

    /// A local T1 from A to B, and a later T2 that also leaves A and goes on from B to C.
    fn local_then_express() -> (tempfile::NamedTempFile, memmap2::Mmap) {
        common::network(
            "R,s,T1\nR,s,T2\n",
            "T1,08:00:00,08:00:00,A,1,0,0\nT1,08:05:00,08:05:00,B,2,0,0\n\
             T2,08:10:00,08:10:00,A,1,0,0\nT2,08:12:00,08:12:00,B,2,0,0\nT2,08:20:00,08:20:00,C,3,0,0\n",
        )
    }

    #[test]
    fn staying_on_the_stop_beats_riding_to_wait_at_the_next_one() {
        let (_file, mmap) = local_then_express();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let query = query(
            QueryLocation::Stop(stop(&consumer, "A")),
            QueryLocation::Stop(stop(&consumer, "C")),
            None,
            None,
        );

        let itinerary = solve(&consumer, query).expect("a route exists");

        assert_eq!(rides(&itinerary), 1, "T2 goes from A to C without a change");
    }

    #[test]
    fn arriving_by_a_time_does_not_add_a_change_either() {
        let (_file, mmap) = local_then_express();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let query = query(
            QueryLocation::Stop(stop(&consumer, "A")),
            QueryLocation::Stop(stop(&consumer, "C")),
            Some("08:30:00"),
            None,
        );

        let itinerary = solve(&consumer, query).expect("a route exists");

        assert_eq!(rides(&itinerary), 1, "T2 goes from A to C without a change");
    }

    /// A direct T1 from A to C that takes 30 minutes, and a change at B that is 10 minutes faster.
    fn direct_or_change() -> (tempfile::NamedTempFile, memmap2::Mmap) {
        common::network(
            "R,s,T1\nR,s,T2\nR,s,T3\n",
            "T1,08:00:00,08:00:00,A,1,0,0\nT1,08:30:00,08:30:00,C,2,0,0\n\
             T2,08:00:00,08:00:00,A,1,0,0\nT2,08:05:00,08:05:00,B,2,0,0\n\
             T3,08:07:00,08:07:00,B,1,0,0\nT3,08:20:00,08:20:00,C,2,0,0\n",
        )
    }

    fn rides_with_boarding_penalty(penalty: f64) -> usize {
        let (_file, mmap) = direct_or_change();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let query = query(
            QueryLocation::Stop(stop(&consumer, "A")),
            QueryLocation::Stop(stop(&consumer, "C")),
            None,
            None,
        )
        .with_departure(common::time("08:00:00"))
        .with_boarding_penalty(penalty);

        rides(&solve(&consumer, query).expect("a route exists"))
    }

    #[test]
    fn a_change_is_made_when_boarding_is_free_and_the_saving_is_ten_minutes() {
        assert_eq!(rides_with_boarding_penalty(0.0), 2, "the change is faster");
    }

    #[test]
    fn a_boarding_penalty_larger_than_the_saving_keeps_the_rider_on_one_vehicle() {
        assert_eq!(
            rides_with_boarding_penalty(1000.0),
            1,
            "the direct trip is slower but changes nothing"
        );
    }

    fn itinerary_with_boarding_penalty(penalty: f64) -> Itinerary {
        let (_file, mmap) = direct_or_change();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let query = query(
            QueryLocation::Stop(stop(&consumer, "A")),
            QueryLocation::Stop(stop(&consumer, "C")),
            None,
            None,
        )
        .with_departure(common::time("08:00:00"))
        .with_boarding_penalty(penalty);
        solve(&consumer, query).expect("a route exists")
    }

    #[test]
    fn an_itinerary_measures_its_time_rides_and_walking() {
        let itinerary = itinerary_with_boarding_penalty(0.0);

        assert_eq!(itinerary.rides(), 2, "T2 and T3");
        assert_eq!(itinerary.changes(), 1, "one change at B");
        assert_eq!(itinerary.duration(), Duration(1200), "08:00 to 08:20");
        assert_eq!(itinerary.walking(), Duration(0), "the stops are the ends");
    }

    #[test]
    fn the_selectors_pick_by_one_measure_each() {
        let faster = itinerary_with_boarding_penalty(0.0);
        let direct = itinerary_with_boarding_penalty(1000.0);
        let journeys = Journeys {
            itineraries: vec![direct, faster].into_boxed_slice(),
        };

        let rides = |found: Option<&Itinerary>| found.map(Itinerary::rides);

        assert_eq!(
            rides(journeys.fastest()),
            Some(2),
            "the change saves ten minutes"
        );
        assert_eq!(
            rides(journeys.earliest_arrival()),
            Some(2),
            "the change arrives first"
        );
        assert_eq!(
            rides(journeys.fewest_changes()),
            Some(1),
            "the direct trip boards once"
        );
    }

    fn journeys_to_c(arrive_by: Option<&str>) -> Vec<usize> {
        let (_file, mmap) = direct_or_change();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let query = query(
            QueryLocation::Stop(stop(&consumer, "A")),
            QueryLocation::Stop(stop(&consumer, "C")),
            arrive_by,
            None,
        );
        let spatial = SpatialGrid::new(&consumer);
        let mut state = State::new(&consumer);

        let journeys = query
            .journeys(&consumer, &spatial, &mut state)
            .expect("a route exists");

        journeys.itineraries.iter().map(Itinerary::rides).collect()
    }

    #[test]
    fn a_search_finds_the_direct_trip_and_the_faster_change() {
        assert_eq!(
            journeys_to_c(None),
            [1, 2],
            "the direct trip arrives at 08:30, the change at 08:20"
        );
    }

    #[test]
    fn a_search_back_from_an_arrival_finds_both_too() {
        assert_eq!(
            journeys_to_c(Some("08:40:00")),
            [1, 2],
            "each leaves at its own time"
        );
    }

    #[test]
    fn a_way_that_is_no_faster_for_more_vehicles_is_left_out() {
        let (_file, mmap) = local_then_express();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let query = query(
            QueryLocation::Stop(stop(&consumer, "A")),
            QueryLocation::Stop(stop(&consumer, "C")),
            None,
            None,
        );
        let spatial = SpatialGrid::new(&consumer);
        let mut state = State::new(&consumer);

        let journeys = query
            .journeys(&consumer, &spatial, &mut state)
            .expect("a route exists");

        assert_eq!(
            journeys.itineraries.len(),
            1,
            "changing at B arrives with the direct ride, so only that is found"
        );
    }

    fn alternatives(count: usize) -> Vec<usize> {
        let (_file, mmap) = direct_or_change();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let query = query(
            QueryLocation::Stop(stop(&consumer, "A")),
            QueryLocation::Stop(stop(&consumer, "C")),
            None,
            None,
        )
        .with_departure(common::time("08:00:00"));
        let spatial = SpatialGrid::new(&consumer);
        let mut state = State::new(&consumer);

        let journeys = query
            .alternatives(count, &consumer, &spatial, &mut state)
            .expect("a route exists");

        journeys.itineraries.iter().map(Itinerary::rides).collect()
    }

    #[test]
    fn alternatives_are_the_best_then_what_avoids_its_main_vehicle() {
        assert_eq!(
            alternatives(5),
            [2, 1],
            "the change first, then the direct trip once T3 is banned, then nothing is left"
        );
    }

    #[test]
    fn alternatives_stop_at_the_count_asked_for() {
        assert_eq!(alternatives(1), [2], "only the best was asked for");
    }
}
