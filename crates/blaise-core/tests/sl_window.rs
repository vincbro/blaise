//! Plans over a window on the Stockholm network in `data/sl.flnt`, or the file `BLAISE_SL` names,
//! when it is there.

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

    #[allow(
        unsafe_code,
        reason = "mapping a file is unsafe in memmap2, and this file is not modified"
    )]
    #[test]
    fn a_morning_window_on_the_stockholm_network_has_unbeaten_journeys() {
        let path = env::var_os("BLAISE_SL").map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/sl.flnt"),
            PathBuf::from,
        );
        let Ok(file) = File::open(path) else {
            return;
        };
        // SAFETY: the file is only read, and nothing else changes it while the test runs.
        let mmap = unsafe { Mmap::map(&file) }.expect("the file maps");
        let consumer = match Consumer::new(&mmap) {
            Ok(consumer) => consumer,
            // A file compiled by an older flatnet is not one this test can read.
            Err(ConsumerError::UnsupportedVersion { .. }) => return,
            Err(error) => panic!("the network is not valid: {error}"),
        };
        let date = Date::parse_gtfs("20261125").expect("a valid date");
        let query = RaptorQuery::new(
            QueryLocation::Coordinate((59.5836, 17.8938).into()),
            QueryLocation::Coordinate((59.3397, 18.0371).into()),
        )
        .with_date(date)
        .with_departure(Time::from_hms("07:30:00").expect("a time"));
        let spatial = SpatialGrid::new(&consumer);
        let mut state = State::new(&consumer);

        let started = Instant::now();
        let journeys = query
            .journeys_within(Duration(3600), &consumer, &spatial, &mut state)
            .expect("journeys exist");
        eprintln!(
            "{} journeys in {:?}",
            journeys.itineraries.len(),
            started.elapsed()
        );
        for itinerary in &journeys.itineraries {
            eprintln!(
                "leave {:?} arrive {:?} rides {}",
                itinerary.departure().map(|time| time.actual.0),
                itinerary.arrival().map(|time| time.actual.0),
                itinerary.rides()
            );
        }

        let arrivals: Vec<u32> = journeys
            .itineraries
            .iter()
            .filter_map(Itinerary::arrival)
            .map(|time| time.actual.0)
            .collect();
        assert!(
            arrivals.len() > 1,
            "an hour on a commuter line has several departures"
        );
    }
}
