use crate::{
    raptor::{MAX_ROUNDS, Parent, ServingRoute, Target, Update},
    repository::Repository,
    shared::{Time, time},
};
use bitvec::prelude::*;
use std::mem;

/// A memory pool for the RAPTOR algorithm's state.
///
/// This allocator pre-allocates all necessary buffers to avoid expensive heap allocations
/// during the hot path of route planning. This is especially useful for long-running
/// services (like web servers) where many short-lived RAPTOR instances are created.
pub struct Allocator {
    /// The best known arrival time at each stop across all rounds.
    pub(crate) tau_star: Vec<Option<Time>>,
    /// Tracks which stops were updated in the current round and need to be explored in the next.
    pub(crate) marked_stops: BitVec<usize, Lsb0>,
    /// Tracks the earliest relevant stop index for each route in the current round.
    pub(crate) active: Vec<u32>,
    pub(crate) active_mask: BitVec<usize, Lsb0>,
    /// Labels from the previous round (k-1).
    pub(crate) prev_labels: Vec<Option<Time>>,
    /// Labels for the current round (k).
    /// We use two arrays to "double-buffer" labels since RAPTOR only ever references the previous round.
    pub(crate) curr_labels: Vec<Option<Time>>,
    /// A flattened 2D matrix [round][stop_index] storing path reconstruction pointers.
    pub(crate) parents: Vec<Option<Parent>>,
    /// Buffer used to batch updates before applying them to the state.
    pub(crate) updates: Vec<Update>,
    /// Total number of stops in the associated repository.
    pub(crate) stop_count: usize,
    /// Pre allocated buffer to skip heap allocations.
    pub(crate) routes_serving_stops: Vec<ServingRoute>,
    /// Holds the target data
    pub(crate) target: Target,
}

impl Allocator {
    /// Creates a new allocator sized for the given repository.
    ///
    /// # Warning
    /// The allocator must be used with the exact same `Repository` it was created for.
    /// Using it with a different repository may cause logic errors or out-of-bounds panics.
    pub fn new(repository: &Repository) -> Self {
        Self {
            tau_star: vec![None; repository.stops.len()],
            marked_stops: bitvec!(usize, Lsb0; 0; repository.stops.len()),
            prev_labels: vec![None; repository.stops.len()],
            curr_labels: vec![None; repository.stops.len()],
            parents: vec![None; repository.stops.len() * MAX_ROUNDS],
            updates: Vec::with_capacity(1024),
            active: vec![u32::MAX; repository.raptor_routes.len()],
            active_mask: bitvec!(usize, Lsb0; 0; repository.raptor_routes.len()),
            stop_count: repository.stops.len(),
            routes_serving_stops: Vec::with_capacity(64),
            target: Target::new(),
        }
    }

    /// Resets the internal buffers to their initial state, allowing the allocator
    /// to be reused for a new search without re-allocating memory.
    pub fn reset(&mut self) {
        self.tau_star.fill(None);
        self.marked_stops.fill(false);
        self.prev_labels.fill(None);
        self.curr_labels.fill(None);
        self.parents.fill(None);
        self.active.fill(u32::MAX);
        self.active_mask.fill(false);
        self.updates.clear();
        self.routes_serving_stops.clear();
        self.target.clear();
    }

    pub(crate) fn run_updates(&mut self, round: usize) {
        self.updates.iter().for_each(|update| {
            let best_time = self.tau_star[update.stop_idx as usize].unwrap_or(time::MAX);
            if update.arrival_time < best_time {
                self.curr_labels[update.stop_idx as usize] = Some(update.arrival_time);
                self.parents[flat_matrix(round, update.stop_idx as usize, self.stop_count)] =
                    Some(update.parent);
                self.tau_star[update.stop_idx as usize] = Some(update.arrival_time);
                self.marked_stops.set(update.stop_idx as usize, true);
            }
        });
        self.updates.clear();
    }

    pub(crate) fn run_updates_reverse(&mut self, round: usize) {
        self.updates.iter().for_each(|update| {
            let best_time = self.tau_star[update.stop_idx as usize].unwrap_or(time::MIN);
            if update.arrival_time > best_time {
                self.curr_labels[update.stop_idx as usize] = Some(update.arrival_time);
                self.parents[flat_matrix(round, update.stop_idx as usize, self.stop_count)] =
                    Some(update.parent);
                self.tau_star[update.stop_idx as usize] = Some(update.arrival_time);
                self.marked_stops.set(update.stop_idx as usize, true);
            }
        });
        self.updates.clear();
    }
    pub(crate) fn get_parents(&self, round: usize) -> &[Option<Parent>] {
        let offset = self.stop_count * round;
        &self.parents[offset..offset + self.stop_count]
    }

    pub(crate) fn swap_labels(&mut self) {
        mem::swap(&mut self.curr_labels, &mut self.prev_labels);
        self.curr_labels.fill(None);
    }
}

pub struct LazyBuffer<T> {
    buffer: Option<Vec<T>>,
    capacity: usize,
}

impl<T> LazyBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: None,
            capacity,
        }
    }

    pub fn push(&mut self, value: T) {
        if let Some(buffer) = &mut self.buffer {
            buffer.push(value);
        } else {
            let mut buffer = Vec::with_capacity(self.capacity);
            buffer.push(value);
            self.buffer = Some(buffer);
        }
    }

    pub fn take(mut self) -> Option<Vec<T>> {
        self.buffer.take()
    }

    pub fn swap(&mut self) -> Vec<T> {
        self.buffer.take().unwrap_or_default()
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
