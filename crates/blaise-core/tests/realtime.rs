//! Plans over a three-stop line, with and without live data.

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use blaise_core::{
        raptor::{
            itinerary::{Itinerary, LegType},
            query::{QueryLocation, RaptorQuery},
            state::State,
        },
        spatial::SpatialGrid,
    };
    use flatnet_compiler::Compiler;
    use flatnet_core::{
        consumer::Consumer,
        models::{Date, StopIndex, Time},
        rt::{Realtime, RealtimeBuilder},
    };
    use gtfs_rt::{
        FeedEntity, FeedHeader, FeedMessage, TripDescriptor, TripUpdate, trip_descriptor,
        trip_update::{StopTimeEvent, StopTimeUpdate},
    };
    use memmap2::Mmap;
    use tempfile::{NamedTempFile, TempDir};

    const DAY: &str = "20300102";

    /// Stops A, B and C, 2 km apart so nobody walks between them, and two trips along the line: T1
    /// leaves A at 08:00 and T2 at 08:30. T2 does not let anyone off at B.
    fn write_feed(dir: &TempDir) {
        let files = [
            (
                "agency.txt",
                "agency_id,agency_name,agency_url,agency_timezone\na,A,http://a.example,Europe/Stockholm\n",
            ),
            (
                "stops.txt",
                "stop_id,stop_name,stop_lat,stop_lon\nA,Alpha,59.00,18.00\nB,Beta,59.02,18.00\nC,Gamma,59.04,18.00\n",
            ),
            (
                "routes.txt",
                "route_id,agency_id,route_short_name,route_type\nR,a,1,3\n",
            ),
            (
                "calendar.txt",
                "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\ns,1,1,1,1,1,1,1,20300101,20301231\n",
            ),
            ("trips.txt", "route_id,service_id,trip_id\nR,s,T1\nR,s,T2\n"),
            (
                "stop_times.txt",
                "trip_id,arrival_time,departure_time,stop_id,stop_sequence,pickup_type,drop_off_type\n\
             T1,08:00:00,08:00:00,A,1,0,0\n\
             T1,08:10:00,08:10:00,B,2,0,0\n\
             T1,08:20:00,08:20:00,C,3,0,0\n\
             T2,08:30:00,08:30:00,A,1,0,0\n\
             T2,08:40:00,08:40:00,B,2,0,1\n\
             T2,08:50:00,08:50:00,C,3,0,0\n",
            ),
        ];
        for (name, text) in files {
            fs::write(dir.path().join(name), text).expect("the feed is writable");
        }
    }

    #[allow(
        unsafe_code,
        reason = "mapping a file is unsafe in memmap2, and this file is not modified"
    )]
    fn network() -> (NamedTempFile, Mmap) {
        let dir = TempDir::new().expect("a temp dir");
        write_feed(&dir);
        let bytes = Compiler::new()
            .compile_gtfs(dir.path())
            .expect("a valid feed");
        let mut file = NamedTempFile::new().expect("a temp file");
        file.write_all(&bytes).expect("the temp file is writable");
        // SAFETY: the file is private to this test and isn't modified while mapped.
        let mmap = unsafe { Mmap::map(file.as_file()) }.expect("the file maps");
        (file, mmap)
    }

    fn trip_update(
        trip_id: &str,
        relationship: trip_descriptor::ScheduleRelationship,
    ) -> TripUpdate {
        TripUpdate {
            trip: TripDescriptor {
                trip_id: Some(trip_id.to_owned()),
                schedule_relationship: Some(relationship as i32),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn feed(updates: Vec<TripUpdate>) -> FeedMessage {
        FeedMessage {
            header: FeedHeader {
                gtfs_realtime_version: "2.0".to_owned(),
                ..Default::default()
            },
            entity: updates
                .into_iter()
                .enumerate()
                .map(|(number, update)| FeedEntity {
                    id: number.to_string(),
                    trip_update: Some(update),
                    ..Default::default()
                })
                .collect(),
        }
    }

    fn date() -> Date {
        Date::parse_gtfs(DAY).expect("a valid date")
    }

    fn stop(consumer: &Consumer<'_>, id: &str) -> StopIndex {
        consumer.stop_by_id(id).expect("the stop exists").index
    }

    fn plan(
        consumer: &Consumer<'_>,
        realtime: Option<&Realtime>,
        from: &str,
        to: &str,
        arrive_by: Option<&str>,
    ) -> Itinerary {
        let spatial = SpatialGrid::new(consumer);
        let mut state = State::new(consumer);
        let from = QueryLocation::Stop(stop(consumer, from));
        let to = QueryLocation::Stop(stop(consumer, to));
        let mut query = RaptorQuery::new(from, to).with_date(date());
        query = match arrive_by {
            Some(time) => query.with_arrival(Time::from_hms(time).expect("a time")),
            None => query.with_departure(Time::from_hms("07:50:00").expect("a time")),
        };
        if let Some(realtime) = realtime {
            query = query.with_realtime(realtime);
        }
        query
            .solve(consumer, &spatial, &mut state)
            .expect("a route exists")
    }

    fn transit_trip(itinerary: &Itinerary, consumer: &Consumer<'_>) -> String {
        let trip = itinerary
            .legs
            .iter()
            .find_map(|leg| match leg.leg_type {
                LegType::Transit(trip) => Some(trip),
                _ => None,
            })
            .expect("the route rides a vehicle");
        consumer.trip_id(consumer.trip(trip).id).to_owned()
    }

    #[test]
    fn rides_the_first_trip_on_schedule() {
        let (_file, mmap) = network();
        let consumer = Consumer::new(&mmap).expect("a valid file");

        let itinerary = plan(&consumer, None, "A", "C", None);

        assert_eq!(transit_trip(&itinerary, &consumer), "T1", "T1 leaves first");
        let leg = itinerary.legs.last().expect("a leg");
        assert_eq!(leg.arrival_time.actual, Time(8 * 3600 + 20 * 60), "08:20");
        assert_eq!(
            leg.arrival_time.scheduled, leg.arrival_time.actual,
            "on time"
        );
    }

    #[test]
    fn a_delay_moves_the_actual_time_and_keeps_the_scheduled_one() {
        let (_file, mmap) = network();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let mut late = trip_update("T1", trip_descriptor::ScheduleRelationship::Scheduled);
        late.stop_time_update = vec![StopTimeUpdate {
            stop_sequence: Some(1),
            // The delay carries on to the stops after it.
            arrival: Some(StopTimeEvent {
                delay: Some(300),
                ..Default::default()
            }),
            departure: Some(StopTimeEvent {
                delay: Some(300),
                ..Default::default()
            }),
            ..Default::default()
        }];
        let mut realtime = Realtime::for_service_date(&consumer, date());
        RealtimeBuilder::new(&consumer).apply([feed(vec![late])].into_iter(), &mut realtime);

        let itinerary = plan(&consumer, Some(&realtime), "A", "C", None);

        assert_eq!(
            transit_trip(&itinerary, &consumer),
            "T1",
            "still the first trip"
        );
        let leg = itinerary.legs.last().expect("a leg");
        assert_eq!(
            leg.arrival_time.scheduled,
            Time(8 * 3600 + 20 * 60),
            "timetable"
        );
        assert_eq!(
            leg.arrival_time.actual,
            Time(8 * 3600 + 25 * 60),
            "five minutes late"
        );
    }

    #[test]
    fn a_canceled_trip_is_not_ridden() {
        let (_file, mmap) = network();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let canceled = trip_update("T1", trip_descriptor::ScheduleRelationship::Canceled);
        let mut realtime = Realtime::for_service_date(&consumer, date());
        RealtimeBuilder::new(&consumer).apply([feed(vec![canceled])].into_iter(), &mut realtime);

        let forward = plan(&consumer, Some(&realtime), "A", "C", None);
        let backward = plan(&consumer, Some(&realtime), "A", "C", Some("09:00:00"));

        assert_eq!(transit_trip(&forward, &consumer), "T2", "T1 does not run");
        assert_eq!(transit_trip(&backward, &consumer), "T2", "T1 does not run");
    }

    #[test]
    fn a_realtime_for_another_day_is_ignored() {
        let (_file, mmap) = network();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let canceled = trip_update("T1", trip_descriptor::ScheduleRelationship::Canceled);
        let other_day = Date::parse_gtfs("20300103").expect("a valid date");
        let mut realtime = Realtime::for_service_date(&consumer, other_day);
        RealtimeBuilder::new(&consumer).apply([feed(vec![canceled])].into_iter(), &mut realtime);

        let itinerary = plan(&consumer, Some(&realtime), "A", "C", None);

        assert_eq!(
            transit_trip(&itinerary, &consumer),
            "T1",
            "the query is for another day"
        );
    }

    #[test]
    fn a_stop_with_no_drop_off_is_not_an_alighting_point() {
        let (_file, mmap) = network();
        let consumer = Consumer::new(&mmap).expect("a valid file");
        // T1 is canceled, so only T2 runs, and it lets nobody off at B.
        let canceled = trip_update("T1", trip_descriptor::ScheduleRelationship::Canceled);
        let mut realtime = Realtime::for_service_date(&consumer, date());
        RealtimeBuilder::new(&consumer).apply([feed(vec![canceled])].into_iter(), &mut realtime);
        let itinerary = plan(&consumer, Some(&realtime), "A", "B", None);

        // T2 is today's only trip left, and it lets nobody off at B. The live data is for today,
        // so tomorrow's T1 is on schedule and is the way to B.
        assert_eq!(
            transit_trip(&itinerary, &consumer),
            "T1",
            "T2 can't be left at B"
        );
        let departure = itinerary.legs[0].departure_time.actual;
        assert!(departure.0 >= 86_400, "T1 runs tomorrow, not today");
    }
}
