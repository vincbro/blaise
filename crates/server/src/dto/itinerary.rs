use ontrack::{
    repository::Repository,
    router::{
        graph::Location,
        itinerary::{Instruction, Itinerary, Leg, Mode},
    },
    shared::geo::Coordinate,
};
use serde::{Deserialize, Serialize};

use crate::dto::{AreaDto, StopDto};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LocationDto {
    Area(AreaDto),
    Stop(StopDto),
    Coordinate(Coordinate),
}

impl LocationDto {
    pub fn from(location: Location, repo: &Repository) -> Option<Self> {
        match location {
            Location::Area(id) => {
                let area = repo.area_by_id(&id)?;
                Some(LocationDto::Area(AreaDto::from(area, repo)))
            }
            Location::Stop(id) => {
                let stop = repo.stop_by_id(&id)?;
                Some(LocationDto::Stop(StopDto::from(stop)))
            }
            Location::Coordinate(coordinate) => Some(LocationDto::Coordinate(coordinate)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItineraryDto {
    pub from: LocationDto,
    pub to: LocationDto,
    pub legs: Vec<LegDto>,
}

impl ItineraryDto {
    pub fn from(itinerary: Itinerary, repo: &Repository) -> Option<Self> {
        let legs: Option<Vec<_>> = itinerary
            .legs
            .into_iter()
            .map(|leg| LegDto::from(leg, repo))
            .collect();
        Some(Self {
            from: LocationDto::from(itinerary.from, repo)?,
            to: LocationDto::from(itinerary.to, repo)?,
            legs: legs?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegDto {
    pub from: LocationDto,
    pub to: LocationDto,
    pub mode: Mode,
    pub instructions: Vec<InstructionDto>,
}

impl LegDto {
    pub fn from(leg: Leg, repo: &Repository) -> Option<Self> {
        let instructions: Option<Vec<_>> = leg
            .instructions
            .into_iter()
            .map(|instruction| InstructionDto::from(instruction, repo))
            .collect();

        Some(Self {
            from: LocationDto::from(leg.from, repo)?,
            to: LocationDto::from(leg.to, repo)?,
            mode: leg.mode,
            instructions: instructions?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstructionDto {
    pub location: LocationDto,
    pub distance_km: f32,
    pub distance_m: f32,
    pub arrival_time: String,
}
impl InstructionDto {
    pub fn from(instruction: Instruction, repo: &Repository) -> Option<Self> {
        Some(Self {
            location: LocationDto::from(instruction.location, repo)?,
            distance_km: instruction.distance.as_kilometers(),
            distance_m: instruction.distance.as_meters(),
            arrival_time: instruction.arrival_time.to_hms_string(),
        })
    }
}
