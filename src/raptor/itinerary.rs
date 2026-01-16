use crate::{
    raptor::{
        location::{Location, Point},
        state::{Parent, ParentType},
    },
    repository::Repository,
    shared::time::Time,
};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Leg {
    pub from: Location,
    pub to: Location,
    pub departue_time: Time,
    pub arrival_time: Time,
    pub stops: Vec<LegStop>,
    pub leg_type: LegType,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum LegType {
    Transit(u32),
    Transfer,
    Walk,
}

impl From<ParentType> for LegType {
    fn from(value: ParentType) -> Self {
        match value {
            ParentType::Transit(trip_idx) => Self::Transit(trip_idx),
            ParentType::Transfer => Self::Transfer,
            ParentType::Walk => Self::Walk,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LegStop {
    pub location: Location,
    pub departure_time: Time,
    pub arrival_time: Time,
}

impl LegStop {
    pub fn generate_stops(parent: &Parent, repository: &Repository) -> Vec<Self> {
        match parent.parent_type {
            ParentType::Transit(trip_idx) => {
                let trip = &repository.trips[trip_idx as usize];
                let stop_times = repository.stop_times_by_trip_idx(trip.index);
                let mut stops = Vec::with_capacity(stop_times.len());
                if let Point::Stop(from_idx) = parent.from
                    && let Point::Stop(to_idx) = parent.to
                {
                    let mut in_trip = false;
                    for stop_time in stop_times {
                        if stop_time.stop_idx == from_idx {
                            in_trip = true;
                        }
                        if in_trip {
                            let stop = &repository.stops[stop_time.stop_idx as usize];
                            stops.push(LegStop {
                                location: Location::Stop(stop.id.clone()),
                                departure_time: stop_time.departure_time,
                                arrival_time: stop_time.arrival_time,
                            });
                            if stop_time.stop_idx == to_idx && in_trip {
                                break;
                            }
                        }
                    }
                }

                stops
            }
            ParentType::Transfer => vec![],
            ParentType::Walk => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Itinerary {
    pub from: Location,
    pub to: Location,
    pub legs: Vec<Leg>,
}

impl Itinerary {
    pub fn new(from: Location, to: Location, path: Vec<Parent>, repository: &Repository) -> Self {
        let legs = path
            .into_iter()
            .map(|parent| {
                let leg_from = point_to_location(&parent.from, repository);
                let leg_to = point_to_location(&parent.to, repository);
                Leg {
                    from: leg_from,
                    to: leg_to,
                    departue_time: parent.departure_time,
                    arrival_time: parent.arrival_time,
                    stops: LegStop::generate_stops(&parent, repository),
                    leg_type: parent.parent_type.into(),
                }
            })
            .collect();
        Self { from, to, legs }
    }
}

fn point_to_location(point: &Point, repository: &Repository) -> Location {
    match point {
        Point::Coordinate(coordinate) => (*coordinate).into(),
        Point::Stop(idx) => {
            let stop = &repository.stops[*idx as usize];
            Location::Stop(stop.id.clone())
        }
    }
}
