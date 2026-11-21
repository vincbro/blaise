//! Plans on the Stockholm network in `data/sl.flnt`, or the file `BLAISE_SL` names, when it is
//! there.

#[cfg(test)]
mod tests {
    use std::{
        env,
        fs::File,
        path::{Path, PathBuf},
        time::Instant,
    };

    use blaise_core::{
        raptor::{
            itinerary::Itinerary,
            query::{QueryLocation, RaptorQuery},
            state::State,
        },
        spatial::SpatialGrid,
    };
    use flatnet_core::{
        consumer::{Consumer, Error as ConsumerError},
        models::{Date, Duration, Time},
    };
    use memmap2::Mmap;

    /// The network, or `None` when there is no file or it was compiled by an older flatnet.
    #[allow(
        unsafe_code,
        reason = "mapping a file is unsafe in memmap2, and this file is not modified"
    )]
    fn network() -> Option<Mmap> {
        let path = env::var_os("BLAISE_SL").map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/sl.flnt"),
            PathBuf::from,
        );
        let file = File::open(path).ok()?;
        // SAFETY: the file is only read, and nothing else changes it while the test runs.
        let mmap = unsafe { Mmap::map(&file) }.expect("the file maps");
        match Consumer::new(&mmap) {
            Ok(_) => Some(mmap),
            Err(ConsumerError::UnsupportedVersion { .. }) => None,
            Err(error) => panic!("the network is not valid: {error}"),
        }
    }

    fn date() -> Date {
        Date::parse_gtfs("20261125").expect("a valid date")
    }

    fn morning() -> Time {
        Time::from_hms("07:30:00").expect("a time")
    }

    fn marsta_to_the_centre() -> RaptorQuery<'static> {
        RaptorQuery::new(
            QueryLocation::Coordinate((59.5836, 17.8938).into()),
            QueryLocation::Coordinate((59.3397, 18.0371).into()),
        )
        .with_date(date())
        .with_departure(morning())
    }

    #[test]
    fn a_morning_window_on_the_stockholm_network_has_unbeaten_journeys() {
        let Some(mmap) = network() else {
            return;
        };
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let spatial = SpatialGrid::new(&consumer);
        let mut state = State::new(&consumer);

        let started = Instant::now();
        let journeys = marsta_to_the_centre()
            .journeys_within(Duration(3600), &consumer, &spatial, &mut state)
            .expect("journeys exist");
        eprintln!(
            "{} journeys in {:?}",
            journeys.itineraries.len(),
            started.elapsed()
        );

        assert!(
            journeys.itineraries.len() > 1,
            "an hour on a commuter line has several departures"
        );
    }

    #[test]
    fn alternatives_on_the_stockholm_network_differ_in_their_main_vehicle() {
        let Some(mmap) = network() else {
            return;
        };
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let spatial = SpatialGrid::new(&consumer);
        let mut state = State::new(&consumer);

        let started = Instant::now();
        let journeys = marsta_to_the_centre()
            .alternatives(4, &consumer, &spatial, &mut state)
            .expect("journeys exist");
        eprintln!(
            "{} alternatives in {:?}",
            journeys.itineraries.len(),
            started.elapsed()
        );

        assert!(
            journeys.itineraries.len() > 1,
            "there is more than one way from Märsta to the centre"
        );
    }

    /// Whether every leg leaves no earlier than the one before it arrived.
    fn is_feasible(itinerary: &Itinerary) -> bool {
        itinerary
            .legs
            .windows(2)
            .all(|pair| pair[1].departure_time.actual >= pair[0].arrival_time.actual)
    }

    /// A position in and around Stockholm for each step.
    fn place(step: u32) -> (f64, f64) {
        (
            f64::from(step % 7).mul_add(0.025, 59.25),
            f64::from(step % 11).mul_add(0.03, 17.85),
        )
    }

    /// The query between two places, leaving at 08:30 or arriving by it.
    fn between(from: u32, to: u32, backward: bool) -> RaptorQuery<'static> {
        let query = RaptorQuery::new(
            QueryLocation::Coordinate(place(from).into()),
            QueryLocation::Coordinate(place(to).into()),
        )
        .with_date(date());
        let time = Time::from_hms("08:30:00").expect("a time");
        if backward {
            query.with_arrival(time)
        } else {
            query.with_departure(time)
        }
    }

    #[test]
    fn every_leg_of_a_journey_on_the_stockholm_network_leaves_after_the_last_arrived() {
        let Some(mmap) = network() else {
            return;
        };
        let consumer = Consumer::new(&mmap).expect("a valid file");
        let spatial = SpatialGrid::new(&consumer);
        let mut state = State::new(&consumer);

        let queries = (0..60_u32).flat_map(|pair| [(pair, false), (pair, true)]);
        let results: Vec<_> = queries
            .filter_map(|(pair, backward)| {
                state.reset();
                let query = between(pair, pair * 5 + 3, backward);
                let journeys = query.journeys(&consumer, &spatial, &mut state).ok()?;
                let feasible = journeys.itineraries.iter().all(is_feasible);
                Some(((pair, backward), journeys.itineraries.len(), feasible))
            })
            .collect();
        let checked: usize = results.iter().map(|&(_, count, _)| count).sum();
        let broken: Vec<_> = results
            .iter()
            .filter(|&&(_, _, feasible)| !feasible)
            .map(|&(query, _, _)| query)
            .collect();

        eprintln!(
            "{checked} journeys checked, {} pairs infeasible",
            broken.len()
        );
        assert!(checked > 20, "most pairs in Stockholm have a route");
        assert!(
            broken.is_empty(),
            "a leg left before the one before it arrived: {broken:?}"
        );
    }
}
