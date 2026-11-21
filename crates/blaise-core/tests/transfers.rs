//! Plans journeys where changing vehicles could be avoided.

mod common;

#[cfg(test)]
mod tests {
    use blaise_core::raptor::query::QueryLocation;
    use flatnet_core::consumer::Consumer;

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
}
