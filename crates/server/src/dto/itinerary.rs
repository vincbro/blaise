use crate::dto::{AreaDto, stop::StopDto};
use blaise::{
    raptor::{
        itinerary::{Itinerary, Leg, LegStop, LegType},
        location::Location,
    },
    repository::Repository,
    shared::{geo::Coordinate, time::Time},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocationDto {
    Area(AreaDto),
    Stop(StopDto),
    Coordinate(Coordinate),
}

impl LocationDto {
    pub fn from(location: Location, repository: &Repository) -> Option<Self> {
        match location {
            Location::Area(id) => repository
                .area_by_id(&id)
                .map(|val| LocationDto::Area(AreaDto::from(val, repository))),
            Location::Stop(id) => repository
                .stop_by_id(&id)
                .map(|val| LocationDto::Stop(StopDto::from(val))),
            Location::Coordinate(coordinate) => Some(LocationDto::Coordinate(coordinate)),
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
    pub leg_type: LegType,
}

impl LegDto {
    pub fn from(leg: Leg, repository: &Repository) -> Option<Self> {
        let stops: Option<Vec<_>> = leg
            .stops
            .into_iter()
            .map(|stop| LegStopDto::from(stop, repository))
            .collect();
        Some(Self {
            from: LocationDto::from(leg.from, repository)?,
            to: LocationDto::from(leg.to, repository)?,
            departue_time: leg.departue_time,
            arrival_time: leg.arrival_time,
            stops: stops?,
            leg_type: leg.leg_type,
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
