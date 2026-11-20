//! Plans journeys over `calendar.txt` and `calendar_dates.txt`, and across midnight.

mod common;

#[cfg(test)]
mod tests {
    use blaise::{
        Error,
        raptor::{itinerary::Itinerary, query::RaptorQuery},
    };
    use gtfs_bin::{
        consumer::Consumer,
        models::{Date, StopIndex, Time},
    };
    use memmap2::Mmap;
    use tempfile::NamedTempFile;

    use crate::common::{self, EVERY_DAY, NO_DAY, solve, stop, time};

    // 2030-01-02 is the test day. The days around it are 20300101 and 20300103.
    const BEFORE: &str = "20300101";
    const DAY: &str = "20300102";
    const AFTER: &str = "20300103";

    fn on(text: &str) -> Date {
        Date::parse_gtfs(text).expect("a valid date")
    }

    /// A trip, T1, along A, B and C that leaves A at the given time and takes ten minutes between
    /// stops.
    fn line(first: &str) -> (String, String) {
        let at = |offset: u32| {
            let start = time(first).0 + offset * 600;
            format!("{:02}:{:02}:00", start / 3600, start % 3600 / 60)
        };
        let times = format!(
            "T1,{0},{0},A,1,0,0\nT1,{1},{1},B,2,0,0\nT1,{2},{2},C,3,0,0\n",
            at(0),
            at(1),
            at(2)
        );
        ("R,s,T1\n".to_owned(), times)
    }

    fn network(calendar: &str, calendar_dates: Option<&str>, first: &str) -> (NamedTempFile, Mmap) {
        let (trips, stop_times) = line(first);
        common::network_on(calendar, calendar_dates, &trips, &stop_times)
    }

    fn departing(consumer: &Consumer<'_>, date: &str, at: &str) -> blaise::Result<Itinerary> {
        let (alpha, gamma) = (stop(consumer, "A"), stop(consumer, "C"));
        solve(
            consumer,
            RaptorQuery::new(alpha.into(), gamma.into())
                .with_date(on(date))
                .with_departure(time(at)),
        )
    }

    fn arriving(consumer: &Consumer<'_>, date: &str, at: &str) -> blaise::Result<Itinerary> {
        let (alpha, gamma): (StopIndex, StopIndex) = (stop(consumer, "A"), stop(consumer, "C"));
        solve(
            consumer,
            RaptorQuery::new(alpha.into(), gamma.into())
                .with_date(on(date))
                .with_arrival(time(at)),
        )
    }

    fn last_arrival(itinerary: &Itinerary) -> Time {
        itinerary.legs.last().expect("a leg").arrival_time.actual
    }

    #[test]
    fn a_service_removed_for_a_day_does_not_run_that_day() {
        let (_file, mmap) = network(EVERY_DAY, Some(&format!("s,{DAY},2\n")), "08:00:00");
        let consumer = Consumer::new(&mmap).expect("a valid file");

        let normal = departing(&consumer, AFTER, "07:50:00").expect("a route");
        let removed = departing(&consumer, DAY, "07:50:00").expect("the next day runs");

        assert_eq!(normal.date, on(AFTER), "an ordinary day");
        assert_eq!(last_arrival(&normal), time("08:20:00"), "the usual trip");
        assert_eq!(
            last_arrival(&removed),
            time("32:20:00"),
            "the holiday has no 08:00, so the journey waits for the next day's"
        );
    }

    #[test]
    fn a_service_added_for_a_day_runs_only_that_day() {
        let (_file, mmap) = network(NO_DAY, Some(&format!("s,{DAY},1\n")), "08:00:00");
        let consumer = Consumer::new(&mmap).expect("a valid file");

        let added = departing(&consumer, DAY, "07:50:00").expect("a route");
        let after = departing(&consumer, AFTER, "07:50:00");
        let before = departing(&consumer, BEFORE, "07:50:00").expect("the next day runs");

        assert_eq!(last_arrival(&added), time("08:20:00"), "the added day");
        assert!(
            matches!(after, Err(Error::NoRoute)),
            "the day after has no service, and the added day is behind it"
        );
        assert_eq!(
            last_arrival(&before),
            time("32:20:00"),
            "the day before waits for the added day"
        );
    }

    #[test]
    fn a_trip_past_midnight_runs_for_the_day_after_its_service() {
        // The service runs on the day before only, and the trip leaves at 24:30, which is 00:30.
        let (_file, mmap) = network(NO_DAY, Some(&format!("s,{BEFORE},1\n")), "24:30:00");
        let consumer = Consumer::new(&mmap).expect("a valid file");

        let itinerary = departing(&consumer, DAY, "00:10:00").expect("the night trip");

        assert_eq!(itinerary.date, on(DAY), "the journey is on the queried day");
        assert_eq!(
            itinerary.legs.first().expect("a leg").departure_time.actual,
            time("00:30:00"),
            "yesterday's 24:30 is 00:30 today"
        );
        assert_eq!(last_arrival(&itinerary), time("00:50:00"), "and gets there");
    }

    #[test]
    fn an_arrival_after_midnight_can_use_a_trip_of_the_day_before() {
        let (_file, mmap) = network(NO_DAY, Some(&format!("s,{BEFORE},1\n")), "24:30:00");
        let consumer = Consumer::new(&mmap).expect("a valid file");

        let itinerary = arriving(&consumer, DAY, "01:00:00").expect("the night trip");

        assert_eq!(itinerary.date, on(DAY), "arriving on the queried day");
        assert_eq!(last_arrival(&itinerary), time("00:50:00"), "in time");
    }

    #[test]
    fn a_journey_that_has_to_leave_the_day_before_starts_on_that_day() {
        // The service runs on the day before only, at 23:00. Arriving at 00:30 means leaving
        // before midnight.
        let (_file, mmap) = network(NO_DAY, Some(&format!("s,{BEFORE},1\n")), "23:00:00");
        let consumer = Consumer::new(&mmap).expect("a valid file");

        let itinerary = arriving(&consumer, DAY, "00:30:00").expect("last night's trip");

        assert_eq!(
            itinerary.date,
            on(BEFORE),
            "the legs count from the day the journey starts"
        );
        assert_eq!(
            itinerary.legs.first().expect("a leg").departure_time.actual,
            time("23:00:00"),
            "it leaves at 23:00"
        );
        assert_eq!(
            last_arrival(&itinerary),
            time("23:20:00"),
            "and gets there before midnight"
        );
    }

    #[test]
    fn a_journey_can_cross_midnight_into_the_next_service_day() {
        let (_file, mmap) = network(NO_DAY, Some(&format!("s,{AFTER},1\n")), "06:00:00");
        let consumer = Consumer::new(&mmap).expect("a valid file");

        let itinerary = departing(&consumer, DAY, "23:00:00").expect("tomorrow's trip");

        assert_eq!(itinerary.date, on(DAY), "it starts on the queried day");
        assert_eq!(
            last_arrival(&itinerary),
            time("30:20:00"),
            "06:20 the next morning, counted past 24:00:00"
        );
    }

    #[test]
    fn a_day_with_no_service_around_it_has_no_route() {
        let (_file, mmap) = network(NO_DAY, Some("s,20300110,1\n"), "08:00:00");
        let consumer = Consumer::new(&mmap).expect("a valid file");

        assert!(
            matches!(departing(&consumer, DAY, "07:50:00"), Err(Error::NoRoute)),
            "the only service is a week away"
        );
    }
}
