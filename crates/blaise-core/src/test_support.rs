//! Builds small networks for tests.

use std::{fmt::Write as _, fs, io::Write as _};

use gtfs_bin_compiler::Compiler;
use gtfs_bin_core::consumer::Consumer;
use memmap2::Mmap;
use tempfile::{NamedTempFile, TempDir};

/// A compiled network kept mapped for as long as the value lives.
#[derive(Debug)]
pub(crate) struct Network {
    _file: NamedTempFile,
    mmap: Mmap,
}

impl Network {
    pub(crate) fn consumer(&self) -> Consumer<'_> {
        Consumer::new(&self.mmap).expect("the compiler wrote a valid file")
    }
}

/// A network with the given stop positions, as `(latitude, longitude)`, and one trip along each
/// list of stops, by position in `stops`. Trips run every day of 2030, a minute between stops.
#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
pub(crate) fn network(stops: &[(f64, f64)], trips: &[Vec<usize>]) -> Network {
    let dir = TempDir::new().expect("a temp dir");
    let mut stops_text = String::from("stop_id,stop_name,stop_lat,stop_lon\n");
    for (index, (latitude, longitude)) in stops.iter().enumerate() {
        writeln!(stops_text, "S{index},Stop {index},{latitude},{longitude}").expect("a string");
    }
    let mut trips_text = String::from("route_id,service_id,trip_id\n");
    let mut times_text = String::from(
        "trip_id,arrival_time,departure_time,stop_id,stop_sequence,pickup_type,drop_off_type\n",
    );
    for (trip, visits) in trips.iter().enumerate() {
        writeln!(trips_text, "R,s,T{trip}").expect("a string");
        for (sequence, stop) in visits.iter().enumerate() {
            let minutes = 8 * 60 + trip * 5 + sequence;
            let time = format!("{:02}:{:02}:00", minutes / 60, minutes % 60);
            writeln!(
                times_text,
                "T{trip},{time},{time},S{stop},{},0,0",
                sequence + 1
            )
            .expect("a string");
        }
    }

    let files = [
        (
            "agency.txt",
            "agency_id,agency_name,agency_url,agency_timezone\na,A,http://a.example,Europe/Stockholm\n".to_owned(),
        ),
        ("stops.txt", stops_text),
        ("routes.txt", "route_id,agency_id,route_short_name,route_type\nR,a,1,3\n".to_owned()),
        (
            "calendar.txt",
            "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\ns,1,1,1,1,1,1,1,20300101,20301231\n".to_owned(),
        ),
        ("trips.txt", trips_text),
        ("stop_times.txt", times_text),
    ];
    for (name, text) in files {
        fs::write(dir.path().join(name), text).expect("the feed is writable");
    }

    let bytes = Compiler::new(dir.path()).compile().expect("a valid feed");
    let mut file = NamedTempFile::new().expect("a temp file");
    file.write_all(&bytes).expect("the temp file is writable");
    // SAFETY: the file is private to the test and isn't modified while mapped.
    let mmap = unsafe { Mmap::map(file.as_file()) }.expect("the file maps");
    Network { _file: file, mmap }
}
