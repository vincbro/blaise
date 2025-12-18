use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{
    repository::Repository,
    router::graph::{Location, SearchStateRef, Transition},
    shared::{geo::Distance, time::Time},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mode {
    Walk,
    Travel, // Split out into diffrent types
    Transfer,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Walk => f.write_str("Walk"),
            Mode::Travel => f.write_str("Travel"),
            Mode::Transfer => f.write_str("Transfer"),
        }
    }
}

impl From<&Transition> for Mode {
    fn from(value: &Transition) -> Self {
        match value {
            Transition::Transit { .. } => Mode::Travel,
            Transition::Walk => Mode::Walk,
            Transition::Transfer { .. } => Mode::Transfer,
            Transition::Genesis => Mode::Walk,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Itinerary {
    pub from: Location,
    pub to: Location,
    pub legs: Vec<Leg>,
}

impl Itinerary {
    pub fn new(
        from: Location,
        to: Location,
        states: &[SearchStateRef],
        repo: &Repository,
    ) -> Option<Self> {
        let mut legs = vec![];
        let mut chunk = vec![&states[0]];
        for i in 1..states.len() {
            let prev = &states[i - 1];
            let curr = &states[i];

            if !prev.transition.is_same_leg(&curr.transition) {
                // Process chunk
                legs.push(Leg::process_chunk(&chunk, repo));
                chunk = vec![prev, curr];
            } else {
                chunk.push(curr);
            }
        }

        if !chunk.is_empty() {
            legs.push(Leg::process_chunk(&chunk, repo));
        }

        Some(Self { from, to, legs })
    }
}

#[derive(Clone, Debug)]
pub struct Leg {
    pub from: Location,
    pub to: Location,
    pub mode: Mode,
    pub instructions: Vec<Instruction>,
}

impl Leg {
    fn get_location_from_state(state: &SearchStateRef, repo: &Repository) -> Location {
        if let Some(stop_idx) = state.stop_idx {
            let stop = &repo.stops[stop_idx as usize];
            Location::Stop(stop.id.clone())
        } else {
            Location::Coordinate(state.coordinate)
        }
    }

    pub fn process_chunk(chunk: &[&SearchStateRef], repo: &Repository) -> Self {
        let from = Self::get_location_from_state(chunk[0], repo);
        let to = Self::get_location_from_state(chunk[chunk.len() - 1], repo);

        let instructions: Vec<Instruction> = chunk
            .iter()
            .map(|state| Instruction {
                location: Self::get_location_from_state(state, repo),
                distance: state.g_distance,
                arrival_time: state.current_time,
            })
            .collect();
        let mode = (&chunk[chunk.len() - 1].transition).into();

        Self {
            from,
            to,
            mode,
            instructions,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instruction {
    pub location: Location,
    pub distance: Distance,
    pub arrival_time: Time,
}
