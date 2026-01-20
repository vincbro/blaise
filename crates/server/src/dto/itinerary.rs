use crate::dto::{AreaDto, stop::StopDto};
use blaise::{
    raptor::{Itinerary, Leg, LegStop, LegType, Location},
    repository::Repository,
    shared::{geo::Coordinate, time::Time},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LocationDto {
    #[serde(rename = "area")]
    Area {
        #[serde(flatten)]
        data: AreaDto,
    },
    #[serde(rename = "stop")]
    Stop {
        #[serde(flatten)]
        data: StopDto,
    },
    #[serde(rename = "coordinate")]
    Coordinate {
        #[serde(flatten)]
        data: Coordinate,
    },
}

impl LocationDto {
    pub fn from(location: Location, repository: &Repository) -> Option<Self> {
        match location {
            Location::Area(id) => repository.area_by_id(&id).map(|val| LocationDto::Area {
                data: AreaDto::from(val, repository),
            }),
            Location::Stop(id) => repository.stop_by_id(&id).map(|val| LocationDto::Stop {
                data: StopDto::from(val),
            }),
            Location::Coordinate(coordinate) => Some(LocationDto::Coordinate { data: coordinate }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegStopDto {
    pub location: LocationDto,
    pub departure_time: Time,
    pub arrival_time: Time,
}

impl LegStopDto {
    pub fn from(leg_stop: LegStop, repository: &Repository) -> Option<Self> {
        Some(Self {
            location: LocationDto::from(leg_stop.location, repository)?,
            departure_time: leg_stop.departure_time,
            arrival_time: leg_stop.arrival_time,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LegDto {
    pub from: LocationDto,
    pub to: LocationDto,
    pub departue_time: Time,
    pub arrival_time: Time,
    pub stops: Vec<LegStopDto>,
    pub mode: Mode,
    pub head_sign: Option<String>,
    pub long_name: Option<String>,
    pub short_name: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Mode {
    // Base modes
    Tram,
    Subway,
    Rail,
    Bus,
    Ferry,
    Walk,
    Transfer,
    Unknown,
}

impl Mode {
    fn from_leg(value: LegType, repository: &Repository) -> Self {
        match value {
            LegType::Transit(trip_idx) => {
                Mode::from(repository.route_by_trip_idx(trip_idx).route_type)
            }
            LegType::Transfer => Mode::Transfer,
            LegType::Walk => Mode::Walk,
        }
    }
}
impl From<i32> for Mode {
    fn from(value: i32) -> Self {
        match value {
            0 | 900..=999 => Mode::Tram,
            1 | 400..=405 => Mode::Subway,
            2 | 100..=199 => Mode::Rail,
            3 | 700..=799 => Mode::Bus,
            4 | 1000..=1099 => Mode::Ferry,
            _ => Mode::Unknown,
        }
    }
}

impl LegDto {
    pub fn from(leg: Leg, repository: &Repository) -> Option<Self> {
        let stops: Option<Vec<_>> = leg
            .stops
            .into_iter()
            .map(|stop| LegStopDto::from(stop, repository))
            .collect();

        let (head_sign, long_name, short_name) = if let LegType::Transit(trip_idx) = leg.leg_type {
            let trip = &repository.trips[trip_idx as usize];
            let head_sign = trip
                .head_sign
                .as_ref()
                .map(|head_sign| head_sign.to_string());
            let route = repository.route_by_trip_idx(trip_idx);
            let long_name = route
                .long_name
                .as_ref()
                .map(|long_name| long_name.to_string());
            let short_name = route
                .short_name
                .as_ref()
                .map(|short_name| short_name.to_string());
            (head_sign, long_name, short_name)
        } else {
            (None, None, None)
        };

        Some(Self {
            from: LocationDto::from(leg.from, repository)?,
            to: LocationDto::from(leg.to, repository)?,
            departue_time: leg.departue_time,
            arrival_time: leg.arrival_time,
            stops: stops?,
            mode: Mode::from_leg(leg.leg_type, repository),
            head_sign,
            long_name,
            short_name,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ItineraryDto {
    pub from: LocationDto,
    pub to: LocationDto,
    pub legs: Vec<LegDto>,
}

impl ItineraryDto {
    pub fn from(itinerary: Itinerary, repository: &Repository) -> Option<Self> {
        let legs: Option<Vec<_>> = itinerary
            .legs
            .into_iter()
            .map(|leg| LegDto::from(leg, repository))
            .collect();
        Some(Self {
            from: LocationDto::from(itinerary.from, repository)?,
            to: LocationDto::from(itinerary.to, repository)?,
            legs: legs?,
        })
    }
}
