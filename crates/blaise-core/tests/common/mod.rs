//! Builds small networks for the journey tests.
#![allow(dead_code, reason = "each test file uses some of the helpers")]

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
    rt::Realtime,
};
use memmap2::Mmap;
use tempfile::{NamedTempFile, TempDir};

pub(crate) const DAY: &str = "20300102";

/// Stops A, B and C, 2 km apart on one meridian so nobody walks between them.
pub(crate) const STOPS: &str = "stop_id,stop_name,stop_lat,stop_lon\nA,Alpha,59.00,18.00\nB,Beta,59.02,18.00\nC,Gamma,59.04,18.00\n";

pub(crate) const STOP_TIMES_HEADER: &str =
    "trip_id,arrival_time,departure_time,stop_id,stop_sequence,pickup_type,drop_off_type\n";

pub(crate) fn network(trips: &str, stop_times: &str) -> (NamedTempFile, Mmap) {
    network_on(EVERY_DAY, None, trips, stop_times)
}

/// A service `s` that runs every day of 2030.
pub(crate) const EVERY_DAY: &str = "s,1,1,1,1,1,1,1,20300101,20301231\n";

/// A service `s` that never runs by its weekly pattern, so only `calendar_dates.txt` decides.
pub(crate) const NO_DAY: &str = "s,0,0,0,0,0,0,0,20300101,20301231\n";

/// Like [`network`], with the given rows for `calendar.txt` and, if any, `calendar_dates.txt`.
#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
pub(crate) fn network_on(
    calendar: &str,
    calendar_dates: Option<&str>,
    trips: &str,
    stop_times: &str,
) -> (NamedTempFile, Mmap) {
    let dir = TempDir::new().expect("a temp dir");
    let files = [
        (
            "agency.txt",
            "agency_id,agency_name,agency_url,agency_timezone\na,A,http://a.example,Europe/Stockholm\n".to_owned(),
        ),
        ("stops.txt", STOPS.to_owned()),
        ("routes.txt", "route_id,agency_id,route_short_name,route_type\nR,a,1,3\n".to_owned()),
        (
            "calendar.txt",
            format!("service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\n{calendar}"),
        ),
        ("trips.txt", format!("route_id,service_id,trip_id\n{trips}")),
        ("stop_times.txt", format!("{STOP_TIMES_HEADER}{stop_times}")),
    ];
    if let Some(rows) = calendar_dates {
        fs::write(
            dir.path().join("calendar_dates.txt"),
            format!("service_id,date,exception_type\n{rows}"),
        )
        .expect("the feed is writable");
    }
    for (name, text) in files {
        fs::write(dir.path().join(name), text).expect("the feed is writable");
    }
    let bytes = Compiler::new()
        .compile_gtfs(dir.path())
        .expect("a valid feed");
    let mut file = NamedTempFile::new().expect("a temp file");
    file.write_all(&bytes).expect("the temp file is writable");
    // SAFETY: the file is private to the test and isn't modified while mapped.
    let mmap = unsafe { Mmap::map(file.as_file()) }.expect("the file maps");
    (file, mmap)
}

pub(crate) fn date() -> Date {
    Date::parse_gtfs(DAY).expect("a valid date")
}

pub(crate) fn time(text: &str) -> Time {
    Time::from_hms(text).expect("a valid time")
}

pub(crate) fn stop(consumer: &Consumer<'_>, id: &str) -> StopIndex {
    consumer.stop_by_id(id).expect("the stop exists").index
}

/// A departure at 07:50, or an arrival by the given time, on the test day.
pub(crate) fn query<'a>(
    from: QueryLocation,
    to: QueryLocation,
    arrive_by: Option<&str>,
    realtime: Option<&'a Realtime>,
) -> RaptorQuery<'a> {
    let query = RaptorQuery::new(from, to).with_date(date());
    let query = match arrive_by {
        Some(at) => query.with_arrival(time(at)),
        None => query.with_departure(time("07:50:00")),
    };
    match realtime {
        Some(realtime) => query.with_realtime(realtime),
        None => query,
    }
}

pub(crate) fn solve(
    consumer: &Consumer<'_>,
    query: RaptorQuery<'_>,
) -> blaise_core::Result<Itinerary> {
    let spatial = SpatialGrid::new(consumer);
    let mut state = State::new(consumer);
    query.solve(consumer, &spatial, &mut state)
}

/// How many times the itinerary rides a vehicle.
pub(crate) fn rides(itinerary: &Itinerary) -> usize {
    itinerary
        .legs
        .iter()
        .filter(|leg| matches!(leg.leg_type, LegType::Transit(_)))
        .count()
}
