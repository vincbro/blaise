//! Plans journeys with boarding rules, connections and walks.

mod common;

#[cfg(test)]
mod tests {
    use blaise_core::{
        Error,
        raptor::{
            itinerary::LegType,
            query::{Location, QueryLocation},
        },
    };
    use flatnet_core::{
        consumer::Consumer,
        models::Coordinate,
        rt::{Realtime, RealtimeBuilder},
    };
    use gtfs_rt::{
        FeedEntity, FeedHeader, FeedMessage, TripDescriptor, TripUpdate, trip_descriptor,
        trip_update::{StopTimeEvent, StopTimeUpdate},
    };

    use crate::common::{self, date, query, rides, solve, stop, time};

    /// A feed where trip `id` is `delay` seconds late from its first stop on.
    fn late(consumer: &Consumer<'_>, id: &str, delay: i32) -> Realtime {
        let event = || {
            Some(StopTimeEvent {
                delay: Some(delay),
                ..Default::default()
            })
        };
        let update = TripUpdate {
            trip: TripDescriptor {
                trip_id: Some(id.to_owned()),
                schedule_relationship: Some(
                    trip_descriptor::ScheduleRelationship::Scheduled as i32,
                ),
                ..Default::default()
            },
            stop_time_update: vec![StopTimeUpdate {
                stop_sequence: Some(1),
                arrival: event(),
                departure: event(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let message = FeedMessage {
            header: FeedHeader {
                gtfs_realtime_version: "2.0".to_owned(),
                ..Default::default()
            },
            entity: vec![FeedEntity {
                id: "0".to_owned(),
                trip_update: Some(update),
                ..Default::default()
            }],
        };
        let mut realtime = Realtime::for_service_date(consumer, date());
        RealtimeBuilder::new(consumer).apply([message].into_iter(), &mut realtime);
        realtime
    }

    /// One trip, T1, along A, B and C at 08:00, 08:10 and 08:20.
    fn one_line() -> (tempfile::NamedTempFile, memmap2::Mmap) {
        common::network(
            "R,s,T1\n",
            "T1,08:00:00,08:00:00,A,1,0,0\nT1,08:10:00,08:10:00,B,2,0,0\nT1,08:20:00,08:20:00,C,3,0,0\n",
        )
    }

    #[test]
    fn a_stop_that_allows_no_pickup_is_not_boarded() {
        let (_file, mmap) = common::network(
            "R,s,T1\n",
            "T1,08:00:00,08:00:00,A,1,1,0\nT1,08:10:00,08:10:00,B,2,0,0\nT1,08:20:00,08:20:00,C,3,0,0\n",
        );
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let (alpha, beta, gamma) = (
            stop(&consumer, "A"),
            stop(&consumer, "B"),
            stop(&consumer, "C"),
        );

        let from_a = solve(&consumer, query(alpha.into(), gamma.into(), None, None));
        let from_a_back = solve(
            &consumer,
            query(alpha.into(), gamma.into(), Some("09:00:00"), None),
        );
        let from_b = solve(&consumer, query(beta.into(), gamma.into(), None, None));

        assert!(
            matches!(from_a, Err(Error::NoRoute)),
            "nobody boards at A going forward"
        );
        assert!(
            matches!(from_a_back, Err(Error::NoRoute)),
            "nobody boards at A going back"
        );
        assert_eq!(
            rides(&from_b.expect("B is boardable")),
            1,
            "B lets passengers on"
        );
    }

    #[test]
    fn a_connection_is_made_on_time_and_missed_when_the_first_trip_is_late() {
        let (_file, mmap) = common::network(
            "R,s,L1\nR,s,L2\n",
            "L1,08:00:00,08:00:00,A,1,0,0\nL1,08:10:00,08:10:00,B,2,0,0\nL2,08:20:00,08:20:00,B,1,0,0\nL2,08:30:00,08:30:00,C,2,0,0\n",
        );
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let (alpha, gamma) = (stop(&consumer, "A"), stop(&consumer, "C"));
        let running_late = late(&consumer, "L1", 900);

        let on_time =
            solve(&consumer, query(alpha.into(), gamma.into(), None, None)).expect("a route");
        let missed = solve(
            &consumer,
            query(alpha.into(), gamma.into(), None, Some(&running_late)),
        );

        assert_eq!(rides(&on_time), 2, "two vehicles, changing at B");
        assert_eq!(
            on_time.legs.last().expect("a leg").arrival_time.actual,
            time("08:30:00"),
            "arrives with the second vehicle"
        );
        let missed = missed.expect("the next day's 08:20 still runs");
        assert_eq!(
            missed.date,
            date(),
            "the journey starts on the day it was asked for"
        );
        assert_eq!(
            missed.legs.last().expect("a leg").arrival_time.actual,
            time("32:30:00"),
            "arriving at B at 08:25 misses the 08:20, so the second vehicle is the one at 08:20 the next day"
        );
    }

    #[test]
    fn an_arrival_by_a_delayed_trip_that_comes_too_late_is_not_used() {
        let (_file, mmap) = one_line();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let (alpha, gamma) = (stop(&consumer, "A"), stop(&consumer, "C"));
        let running_late = late(&consumer, "T1", 300);

        let on_time = solve(
            &consumer,
            query(alpha.into(), gamma.into(), Some("08:22:00"), None),
        );
        let late_trip = solve(
            &consumer,
            query(
                alpha.into(),
                gamma.into(),
                Some("08:22:00"),
                Some(&running_late),
            ),
        );
        let late_but_enough = solve(
            &consumer,
            query(
                alpha.into(),
                gamma.into(),
                Some("08:27:00"),
                Some(&running_late),
            ),
        );

        assert!(on_time.is_ok(), "08:20 is in time for 08:22");
        let earlier_day = late_trip.expect("the day before still runs");
        assert_eq!(
            earlier_day.date.0 + 1,
            date().0,
            "08:25 is too late for 08:22, so the journey is the day before"
        );
        assert_eq!(
            earlier_day.legs.last().expect("a leg").arrival_time.actual,
            time("08:20:00"),
            "and the delay does not follow it there"
        );
        let arrival = late_but_enough
            .expect("08:25 is in time for 08:27")
            .legs
            .last()
            .expect("a leg")
            .arrival_time;
        assert_eq!(
            arrival.actual,
            time("08:25:00"),
            "the delay shows in the time"
        );
    }

    #[test]
    fn a_journey_to_a_coordinate_ends_with_a_walk_in_both_directions() {
        let (_file, mmap) = one_line();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let alpha = stop(&consumer, "A");
        // About 200 m north of C.
        let destination = QueryLocation::Coordinate(Coordinate::new(59.0418, 18.0));

        let forward = solve(
            &consumer,
            query(alpha.into(), destination.clone(), None, None),
        )
        .expect("a route");
        let back = solve(
            &consumer,
            query(alpha.into(), destination, Some("09:00:00"), None),
        )
        .expect("a route");

        for (itinerary, direction) in [(forward, "forward"), (back, "back")] {
            let last = itinerary.legs.last().expect("a leg");
            assert!(
                matches!(last.leg_type, LegType::Walk),
                "{direction}: the last leg walks"
            );
            assert!(
                matches!(itinerary.to, Location::Coordinate(_)),
                "{direction}: it ends at the coordinate"
            );
            assert!(
                last.arrival_time.actual <= time("09:00:00"),
                "{direction}: it is in time"
            );
        }
    }
}
