use crate::engine::{
    Engine,
    geo::Distance,
    routing::{
        Location,
        graph::{SearchState, SearchStateRef},
    },
};

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
        engine: &Engine,
    ) -> Option<Self> {
        let mut legs = vec![];
        let mut chunk = vec![&states[0]];
        for i in 1..states.len() {
            let prev = &states[i - 1];
            let curr = &states[i];

            if prev.transition.is_same_leg(&curr.transition) {
                // Process chunk
                legs.push(Leg::process_chunk(&chunk, engine));
                chunk = vec![&curr];
            } else {
                chunk.push(&curr);
            }
        }

        Some(Self { from, to, legs })
    }
}

#[derive(Clone, Debug)]
pub struct Leg {
    pub from: Location,
    pub to: Location,
    pub instructions: Vec<Instruction>,
}

impl Leg {
    pub fn process_chunk(chunk: &[&SearchStateRef], engine: &Engine) -> Self {
        Self {
            from: Location::Area("TEST".into()),
            to: Location::Area("TEST".into()),
            instructions: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instruction {
    pub location: Location,
    pub distance: Distance,
    pub arrival_time: usize,
}
