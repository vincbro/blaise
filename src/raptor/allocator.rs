use std::mem;

// use rayon::iter::{IntoParallelIterator, ParallelExtend};
use rayon::prelude::*;

use crate::{
    raptor::{MAX_ROUNDS, Parent, Update},
    repository::Repository,
    shared::Time,
};

pub struct Allocator {
    pub(crate) tau_star: Vec<Option<Time>>,
    pub(crate) marked_stops: Vec<bool>,
    // We use 2 arrays that we then switch every round since you only
    // ever look at the current and last rounds in raptor,
    // so keeping a full record does nothing.
    pub(crate) prev_labels: Vec<Option<Time>>,
    pub(crate) curr_labels: Vec<Option<Time>>,
    // Moving parents to a single
    pub(crate) parents: Vec<Option<Parent>>,
    // Holds a buffer off updates
    pub(crate) updates: Vec<Update>,
    pub(crate) stop_count: usize,
}

impl Allocator {
    pub fn new(repository: &Repository) -> Self {
        Self {
            tau_star: vec![None; repository.stops.len()],
            marked_stops: vec![false; repository.stops.len()],
            prev_labels: vec![None; repository.stops.len()],
            curr_labels: vec![None; repository.stops.len()],
            parents: vec![None; repository.stops.len() * MAX_ROUNDS],
            updates: Vec::with_capacity(1024),
            stop_count: repository.stops.len(),
        }
    }

    pub fn reset(&mut self) {
        self.tau_star.fill(None);
        self.marked_stops.fill(false);
        self.prev_labels.fill(None);
        self.curr_labels.fill(None);
        self.parents.fill(None);
        self.updates.clear();
    }

    pub(crate) fn run_updates(&mut self, round: usize) {
        self.updates.iter().for_each(|update| {
            let best_time = self.tau_star[update.stop_idx as usize].unwrap_or(u32::MAX.into());
            if update.arrival_time < best_time {
                self.curr_labels[update.stop_idx as usize] = Some(update.arrival_time);
                self.parents[flat_matrix(round, update.stop_idx as usize, self.stop_count)] =
                    Some(update.parent);
                self.tau_star[update.stop_idx as usize] = Some(update.arrival_time);
                self.marked_stops[update.stop_idx as usize] = true;
            }
        });
        self.updates.clear();
    }

    pub(crate) fn get_parents(&self, round: usize) -> &[Option<Parent>] {
        let offset = self.stop_count * round;
        &self.parents[offset..offset + self.stop_count]
    }

    pub(crate) fn get_marked_stops(&self) -> Vec<usize> {
        self.marked_stops
            .par_iter()
            .enumerate()
            .filter_map(|(i, &m)| m.then_some(i))
            .collect()
    }

    pub(crate) fn swap_labels(&mut self) {
        mem::swap(&mut self.curr_labels, &mut self.prev_labels);
        self.curr_labels.fill(None);
    }
}

/// Converts a (round, stop_index) coordinate into a flat index
/// for the 1D parents/labels arrays.
#[inline(always)] // Hint to compiler to inline for performance
pub(crate) fn flat_matrix(outer: usize, inner: usize, count: usize) -> usize {
    (outer * count) + inner
}

#[test]
fn flat_matrix_test() {
    let a = flat_matrix(0, 0, 10);
    let b = flat_matrix(0, 1, 10);
    assert_eq!(a + 1, b);

    let a = flat_matrix(1, 0, 10);
    let b = flat_matrix(1, 1, 10);
    assert_eq!(a + 1, b);

    let a = flat_matrix(2, 0, 10);
    let b = flat_matrix(2, 1, 10);
    assert_eq!(a + 1, b);

    let a = flat_matrix(0, 0, 10);
    let b = flat_matrix(1, 0, 10);
    assert_eq!(a + 10, b);
}
