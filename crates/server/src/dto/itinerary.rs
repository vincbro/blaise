use ontrack::engine::{
    Engine,
    geo::Coordinate,
    routing::{
        graph::Location,
        itinerary::{Instruction, Itinerary, Leg, Mode},
    },
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
    pub fn from(location: Location, engine: &Engine) -> Option<Self> {
        match location {
            Location::Area(id) => {
                let area = engine.area_by_id(&id)?;
                Some(LocationDto::Area(AreaDto::from(area, engine)))
            }
            Location::Stop(id) => {
                let stop = engine.stop_by_id(&id)?;
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
    pub fn from(itinerary: Itinerary, engine: &Engine) -> Option<Self> {
        let legs: Option<Vec<_>> = itinerary
            .legs
            .into_iter()
            .map(|leg| LegDto::from(leg, engine))
            .collect();
        Some(Self {
            from: LocationDto::from(itinerary.from, engine)?,
            to: LocationDto::from(itinerary.to, engine)?,
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
    pub fn from(leg: Leg, engine: &Engine) -> Option<Self> {
        let instructions: Option<Vec<_>> = leg
            .instructions
            .into_iter()
            .map(|instruction| InstructionDto::from(instruction, engine))
            .collect();

        Some(Self {
            from: LocationDto::from(leg.from, engine)?,
            to: LocationDto::from(leg.to, engine)?,
            mode: leg.mode,
            instructions: instructions?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstructionDto {
    pub location: LocationDto,
    pub distance_km: f64,
    pub distance_m: f64,
    pub arrival_time: usize,
}
impl InstructionDto {
    pub fn from(instruction: Instruction, engine: &Engine) -> Option<Self> {
        Some(Self {
            location: LocationDto::from(instruction.location, engine)?,
            distance_km: instruction.distance.as_kilometers(),
            distance_m: instruction.distance.as_meters(),
            arrival_time: instruction.arrival_time,
        })
    }
}
