use std::mem;

use crate::{
    raptor::{MAX_ROUNDS, location::Point},
    repository::Repository,
    shared::time::Time,
};
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
    // We use 2 arrays that we then switch every round since you only
    // ever look at the current and last rounds in raptor,
    // so keeping a full record does nothing.
    pub prev_labels: Vec<Option<Time>>,
    pub curr_labels: Vec<Option<Time>>,
    // Moving parents to a single
    pub parents: Vec<Option<Parent>>,
    // Holds a buffer off updates
    pub updates: Vec<Update>,
    stop_count: usize,
}

impl State {
    pub fn new(repository: &Repository) -> Self {
        Self {
            tau_star: vec![None; repository.stops.len()],
            marked: vec![false; repository.stops.len()],
            prev_labels: vec![None; repository.stops.len()],
            curr_labels: vec![None; repository.stops.len()],
            parents: vec![None; repository.stops.len() * MAX_ROUNDS],
            updates: Vec::with_capacity(1024),
            stop_count: repository.stops.len(),
        }
    }

    pub fn apply_updates(&mut self, round: usize) {
        self.updates.iter().for_each(|update| {
            let best_time = self.tau_star[update.stop_idx as usize].unwrap_or(u32::MAX.into());
            if update.arrival_time < best_time {
                self.curr_labels[update.stop_idx as usize] = Some(update.arrival_time);
                self.parents[flat_matrix(round, update.stop_idx, self.stop_count)] =
                    Some(update.parent);
                self.tau_star[update.stop_idx as usize] = Some(update.arrival_time);
                self.marked[update.stop_idx as usize] = true;
            }
        });
        self.updates.clear();
    }

    pub fn marked_stops(&self) -> Vec<usize> {
        self.marked
            .par_iter()
            .enumerate()
            .filter_map(|(i, &m)| m.then_some(i))
            .collect()
    }

    pub fn switch_labels(&mut self) {
        mem::swap(&mut self.curr_labels, &mut self.prev_labels);
        self.curr_labels.fill(None);
    }
}

/// Converts a (round, stop_index) coordinate into a flat index
/// for the 1D parents/labels arrays.
#[inline(always)] // Hint to compiler to inline for performance
pub fn flat_matrix(outer: usize, inner: u32, count: usize) -> usize {
    (outer * count) + inner as usize
}
