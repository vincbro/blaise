use crate::{repository::Repository, router::location::Point, shared::time::Time};
use rayon::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum ParentType {
    Transit(u32),
    Transfer,
    Walk,
}

impl ParentType {
    pub fn is_transit(&self) -> bool {
        matches!(self, ParentType::Transit(_))
    }

    pub fn is_transfer(&self) -> bool {
        matches!(self, ParentType::Transfer)
    }

    pub fn is_walk(&self) -> bool {
        matches!(self, ParentType::Walk)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Parent {
    pub from: Point,
    pub to: Point,
    pub parent_type: ParentType,
    pub departure_time: Time,
    pub arrival_time: Time,
}

impl Parent {
    pub fn new_transit(
        from: Point,
        to: Point,
        trip: u32,
        departure_time: Time,
        arrival_time: Time,
    ) -> Self {
        Self {
            from,
            to,
            parent_type: ParentType::Transit(trip),
            departure_time,
            arrival_time,
        }
    }
    pub fn new_transfer(from: Point, to: Point, departure_time: Time, arrival_time: Time) -> Self {
        Self {
            from,
            to,
            parent_type: ParentType::Transfer,
            departure_time,
            arrival_time,
        }
    }
    pub fn new_walk(from: Point, to: Point, departure_time: Time, arrival_time: Time) -> Self {
        Self {
            from,
            to,
            parent_type: ParentType::Walk,
            departure_time,
            arrival_time,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Update {
    pub stop_idx: u32,
    pub arrival_time: Time,
    pub parent: Parent,
}

impl Update {
    pub fn new(stop_idx: u32, arrival_time: Time, parent: Parent) -> Self {
        Self {
            stop_idx,
            arrival_time,
            parent,
        }
    }
}

#[derive(Debug, Default)]
pub struct State {
    pub tau_star: Vec<Option<Time>>,
    pub marked: Vec<bool>,
    pub labels: Vec<Vec<Option<Time>>>,
    pub parents: Vec<Vec<Option<Parent>>>,
}

impl State {
    pub fn new(repository: &Repository) -> Self {
        Self {
            tau_star: vec![None; repository.stops.len()],
            marked: vec![false; repository.stops.len()],
            labels: vec![],
            parents: vec![],
        }
    }

    pub fn apply_updates(&mut self, round: usize, updates: Vec<Update>) {
        updates.into_iter().for_each(|update| {
            let best_time = self.tau_star[update.stop_idx as usize].unwrap_or(u32::MAX.into());
            if update.arrival_time < best_time {
                self.labels[round][update.stop_idx as usize] = Some(update.arrival_time);
                self.parents[round][update.stop_idx as usize] = Some(update.parent);
                self.tau_star[update.stop_idx as usize] = Some(update.arrival_time);
                self.marked[update.stop_idx as usize] = true;
            }
        })
    }

    pub fn marked_stops(&self) -> Vec<usize> {
        self.marked
            .par_iter()
            .enumerate()
            .filter_map(|(i, &m)| m.then_some(i))
            .collect()
    }
}
