//! Plans a journey over a compiled GTFS network from the command line.

use std::{env, fs, fs::File, time::Instant};

use anyhow::Context;
use blaise_core::{
    raptor::{
        self, LiveTime,
        itinerary::{Itinerary, Leg, LegType},
        query::{Location, QueryLocation, RaptorQuery},
        state::State,
    },
    spatial::SpatialGrid,
};
use flatnet_core::{
    consumer::Consumer,
    rt::{self, Realtime, RealtimeBuilder},
};
use memmap2::MmapOptions;

const RULE: &str = "=========================================================";

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .context("usage: blaise <network.flnt> [realtime.pb ...]")?;
    let file = File::open(path).with_context(|| format!("failed to open {path}"))?;
    // SAFETY: the file is only read, and nothing here modifies it while it is mapped.
    let mmap = unsafe { MmapOptions::new().map(&file) }
        .with_context(|| format!("failed to map {path}"))?;

    let consumer =
        Consumer::new(&mmap).with_context(|| format!("{path} is not a valid network file"))?;
    let spatial = SpatialGrid::new(&consumer);

    let query = RaptorQuery::new(
        QueryLocation::Coordinate((59.5836, 17.8938).into()),
        QueryLocation::Coordinate((59.3397, 18.0371).into()),
    );

    // Every file after the network is a GTFS-realtime feed, applied in order.
    let mut realtime = Realtime::for_service_date(&consumer, query.date);
    let builder = RealtimeBuilder::new(&consumer);
    for feed_path in &args[2..] {
        let bytes = fs::read(feed_path).with_context(|| format!("failed to read {feed_path}"))?;
        let message =
            rt::decode_feed(&bytes).with_context(|| format!("failed to decode {feed_path}"))?;
        let report = builder.apply([message].into_iter(), &mut realtime);
        println!("{feed_path}: {report:?}");
    }
    let query = if args.len() > 2 {
        query.with_realtime(&realtime)
    } else {
        query
    };

    let mut state = State::new(&consumer);
    let started = Instant::now();
    let itinerary =
        raptor::solve(query, &consumer, &spatial, &mut state).context("no route was found")?;
    println!("Solving route query took {:?}", started.elapsed());

    print_itinerary(&itinerary, &consumer);
    Ok(())
}

fn format_location(location: &Location, consumer: &Consumer<'_>) -> String {
    match location {
        Location::Stop(stop_index) => {
            let stop = consumer.stop(*stop_index);
            consumer
                .stop_info(*stop_index)
                .name
                .as_option()
                .map_or_else(
                    || consumer.stop_id(stop.id).to_owned(),
                    |name| consumer.string(name).to_owned(),
                )
        }
        Location::Coordinate(coordinate) => {
            format!(
                "Location ({}, {})",
                coordinate.latitude_f64(),
                coordinate.longitude_f64()
            )
        }
    }
}

/// ` (+2 min)` when the vehicle runs late, ` (-1 min)` when early, nothing when on time.
fn format_delay(time: LiveTime) -> String {
    let delay = i64::from(time.actual.0) - i64::from(time.scheduled.0);
    if delay == 0 {
        String::new()
    } else {
        format!(" ({:+} min)", delay / 60)
    }
}

fn print_itinerary(itinerary: &Itinerary, consumer: &Consumer<'_>) {
    // `ESC c` resets the terminal, so the itinerary replaces whatever the last run printed.
    print!("\x1bc");
    println!("\n{RULE}\n🗺️  \x1b[1mJOURNEY ITINERARY\x1b[0m\n{RULE}\n");

    let (Some(first), Some(last)) = (itinerary.legs.first(), itinerary.legs.last()) else {
        println!("No travel needed. You are already at your destination!");
        return;
    };
    let end_time = last.arrival_time.actual;

    println!(
        "📍 \x1b[1mFrom:\x1b[0m  {}",
        format_location(&itinerary.from, consumer)
    );
    println!(
        "📍 \x1b[1mTo:\x1b[0m    {}",
        format_location(&itinerary.to, consumer)
    );
    println!(
        "⏱️ \x1b[1mTime:\x1b[0m  {} -> {end_time}",
        first.departure_time.actual
    );
    println!("---------------------------------------------------------\n");

    for leg in &itinerary.legs {
        match leg.leg_type {
            LegType::Transit(_) => print_transit(leg, consumer),
            LegType::Transfer => print_foot(leg, "🔄", "\x1b[1;33mTRANSFER\x1b[0m", consumer),
            LegType::Walk => print_foot(leg, "🚶", "\x1b[1;32mWALK\x1b[0m", consumer),
            LegType::Origin => {
                print_foot(leg, "🚶", "\x1b[1;32mSTARTING WALK\x1b[0m", consumer);
            }
        }
    }

    println!("{RULE}\n🎉 \x1b[1mArrived at Destination at {end_time}\x1b[0m\n{RULE}\n");
}

fn print_transit(leg: &Leg, consumer: &Consumer<'_>) {
    println!(
        "🚌 \x1b[1;34mTRANSIT\x1b[0m{}",
        format_delay(leg.departure_time)
    );

    let departure = leg.departure_time.actual;
    let arrival = leg.arrival_time.actual;
    if leg.stops.is_empty() {
        println!(
            "  {departure} ┌ Board at \x1b[1m{}\x1b[0m",
            format_location(&leg.from, consumer)
        );
        println!("         │");
        println!(
            "  {arrival} └ Alight at \x1b[1m{}\x1b[0m\n",
            format_location(&leg.to, consumer)
        );
        return;
    }

    let last = leg.stops.len() - 1;
    for (position, stop) in leg.stops.iter().enumerate() {
        let name = format_location(&stop.location, consumer);
        match position {
            0 => println!(
                "  {} ┌ Board at \x1b[1m{name}\x1b[0m",
                stop.departure_time.actual
            ),
            _ if position == last => println!(
                "  {} └ Alight at \x1b[1m{name}\x1b[0m\n",
                stop.arrival_time.actual
            ),
            _ => println!("  {} ├  {name}", stop.arrival_time.actual),
        }
    }
}

fn print_foot(leg: &Leg, icon: &str, title: &str, consumer: &Consumer<'_>) {
    println!("{icon} {title}");
    println!(
        "  {} ┌ Leave \x1b[1m{}\x1b[0m",
        leg.departure_time.actual,
        format_location(&leg.from, consumer)
    );
    println!(
        "  {} └ Arrive at \x1b[1m{}\x1b[0m\n",
        leg.arrival_time.actual,
        format_location(&leg.to, consumer)
    );
}
