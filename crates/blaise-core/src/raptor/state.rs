use std::mem;

use flatnet_core::{
    consumer::Consumer,
    models::{Duration, Optional, Sentinel, StopIndex, Time, TripPatternIndex},
};

use crate::raptor::{
    MAX_ROUNDS, PatternPosition,
    pareto::{ParetoFront, ParetoLabel},
    update::{Parent, Update},
};

/// The most stops a network can have for `check_invariants` to read every table on each call.
const CHECK_LIMIT: usize = 2_000;

/// The working memory of a search, reused between queries.
///
/// After every public call:
/// - The per-stop tables have one entry per stop of the consumer it was built from.
/// - There is one parent slot per stop for each round, plus one spare.
/// - Every front is a set of labels none of which beats another.
/// - A marked stop has a label.
/// - The target has a best stop exactly when it has a best round, and that round is one of the
///   search's.
#[derive(Debug)]
pub struct State {
    /// Best labels per stop, accumulated across all rounds.
    tau_star: Box<[ParetoFront]>,
    /// Labels found in the current round.
    current_labels: Box<[ParetoFront]>,
    /// Labels found in the previous round, used to find new boarding opportunities.
    previous_labels: Box<[ParetoFront]>,

    marked_stops: Box<[bool]>,
    /// For each trip pattern, the first position a marked stop lets the scan start from.
    active_trip_patterns: Box<[Optional<PatternPosition>]>,

    target: Target,
    /// The stops that lead to the target, and how long the rest of the way takes.
    target_stops: Vec<(StopIndex, Duration)>,

    parents: Box<[Option<Parent>]>,
    stop_count: usize,
}

impl State {
    #[must_use]
    pub fn new(consumer: &Consumer<'_>) -> Self {
        let stop_count = consumer.stops().len();
        let fronts = || (0..stop_count).map(|_| ParetoFront::new()).collect();
        let state = Self {
            tau_star: fronts(),
            current_labels: fronts(),
            previous_labels: fronts(),
            marked_stops: vec![false; stop_count].into_boxed_slice(),
            active_trip_patterns: vec![
                Optional::new(PatternPosition::NONE);
                consumer.trip_patterns().len()
            ]
            .into_boxed_slice(),
            target: Target::new(),
            target_stops: Vec::new(),
            parents: vec![None; stop_count * MAX_ROUNDS + 1].into_boxed_slice(),
            stop_count,
        };
        state.check_invariants();
        state
    }

    /// Forgets the last search so the state can serve another.
    pub fn reset(&mut self) {
        for fronts in [
            &mut self.tau_star,
            &mut self.current_labels,
            &mut self.previous_labels,
        ] {
            fronts.iter_mut().for_each(ParetoFront::clear);
        }
        self.marked_stops.fill(false);
        self.active_trip_patterns
            .fill(Optional::new(PatternPosition::NONE));
        self.target.clear();
        self.target_stops.clear();
        self.parents.fill(None);
        self.check_invariants();
    }

    /// Starts the search at a stop with a label of its own.
    pub(crate) fn seed(&mut self, stop: StopIndex, label: ParetoLabel, is_arrival: bool) {
        self.marked_stops[stop.as_usize()] = true;
        self.current_labels[stop.as_usize()].add(label, is_arrival);
        self.tau_star[stop.as_usize()].add(label, is_arrival);
        let slot = self.parent_slot(0, stop);
        self.parents[slot] = Some(Parent::Origin);
        self.check_invariants();
    }

    /// Lets the search end at a stop, with `rest_of_way` left to get to the destination.
    pub(crate) fn add_target(&mut self, stop: StopIndex, rest_of_way: Duration) {
        self.target_stops.push((stop, rest_of_way));
        self.check_invariants();
    }

    /// Whether a label is beaten by one the stop already has.
    pub(crate) fn is_dominated_at(
        &self,
        stop: StopIndex,
        time: Time,
        cost: f32,
        is_arrival: bool,
    ) -> bool {
        self.tau_star[stop.as_usize()].is_dominated(time, cost, is_arrival)
    }

    /// Whether a label is beaten by one the stop already has, or by one that already reaches the
    /// target.
    pub(crate) fn is_dominated(
        &self,
        stop: StopIndex,
        time: Time,
        cost: f32,
        is_arrival: bool,
    ) -> bool {
        self.is_dominated_at(stop, time, cost, is_arrival)
            || self.target.labels.is_dominated(time, cost, is_arrival)
    }

    pub(crate) fn previous_labels(&self, stop: StopIndex) -> &ParetoFront {
        &self.previous_labels[stop.as_usize()]
    }

    pub(crate) fn current_labels(&self, stop: StopIndex) -> &ParetoFront {
        &self.current_labels[stop.as_usize()]
    }

    pub(crate) fn has_marked_stops(&self) -> bool {
        self.marked_stops.iter().any(|&marked| marked)
    }

    /// One flag per stop, set when a label was found for it since the last round began.
    pub(crate) fn marked_stops(&self) -> &[bool] {
        &self.marked_stops
    }

    pub(crate) fn active_trip_patterns(&self) -> &[Optional<PatternPosition>] {
        &self.active_trip_patterns
    }

    /// The label with the lowest cost at a stop.
    pub(crate) fn cheapest_label(&self, stop: StopIndex) -> Option<ParetoLabel> {
        self.tau_star[stop.as_usize()].cheapest()
    }

    /// The label with the lowest cost that reaches the target.
    pub(crate) fn cheapest_target_label(&self) -> Option<ParetoLabel> {
        self.target.labels.cheapest()
    }

    /// The stop and round of the cheapest way to the target, if there is one.
    pub(crate) fn best_target(&self) -> Option<(StopIndex, usize)> {
        Some((self.target.best_stop.as_option()?, self.target.best_round?))
    }

    pub(crate) fn parent(&self, round: usize, stop: StopIndex) -> Option<Parent> {
        self.parents[self.parent_slot(round, stop)]
    }

    /// Makes the labels of the last round the ones to board from, and picks where on each trip
    /// pattern the next scan starts: the first marked stop going forward, the last going back.
    pub(crate) fn begin_round(&mut self, consumer: &Consumer<'_>, is_arrival: bool) {
        mem::swap(&mut self.current_labels, &mut self.previous_labels);
        self.current_labels.iter_mut().for_each(ParetoFront::clear);
        self.active_trip_patterns
            .fill(Optional::new(PatternPosition::NONE));

        let marked = self
            .marked_stops
            .iter()
            .enumerate()
            .filter(|(_, marked)| **marked)
            .map(|(index, _)| StopIndex::from(index));
        for stop in marked {
            activate_patterns_at(&mut self.active_trip_patterns, consumer, stop, is_arrival);
        }
        self.marked_stops.fill(false);
        self.check_invariants();
    }

    /// Keeps the updates no label already beats, and marks their stops.
    pub(crate) fn apply_updates(&mut self, round: usize, is_arrival: bool, updates: &[Update]) {
        for update in updates {
            let label = ParetoLabel::new(update.time, update.cost);
            if self.is_dominated(update.stop, update.time, update.cost, is_arrival) {
                continue;
            }

            // Whether this label becomes the cheapest the stop has this round.
            let previous_best = self.current_labels[update.stop.as_usize()].best_cost();
            if !self.tau_star[update.stop.as_usize()].add(label, is_arrival) {
                continue;
            }
            self.current_labels[update.stop.as_usize()].add(label, is_arrival);
            if update.cost <= previous_best {
                let slot = self.parent_slot(round, update.stop);
                self.parents[slot] = Some(update.parent);
            }
            self.marked_stops[update.stop.as_usize()] = true;
        }
        self.check_invariants();
    }

    /// Adds the labels of this round at the target stops to the labels that reach the target.
    pub(crate) fn record_targets(&mut self, round: usize, is_arrival: bool, walk_penalty: f64) {
        for &(stop, rest_of_way) in &self.target_stops {
            for label in self.current_labels[stop.as_usize()].iter() {
                let reached = label.after_walk(rest_of_way, walk_penalty, is_arrival);
                self.target.offer(stop, round, reached, is_arrival);
            }
        }
        self.check_invariants();
    }

    /// Panics in debug builds if an invariant doesn't hold. A network with many stops is skipped,
    /// because the check reads every table.
    fn check_invariants(&self) {
        if !cfg!(debug_assertions) || self.stop_count > CHECK_LIMIT {
            return;
        }
        let tables = [&self.tau_star, &self.current_labels, &self.previous_labels];
        for table in tables {
            assert_eq!(
                table.len(),
                self.stop_count,
                "a label table has a front for every stop"
            );
            table.iter().for_each(ParetoFront::check_invariants);
        }
        assert_eq!(
            self.marked_stops.len(),
            self.stop_count,
            "every stop is marked or not"
        );
        assert_eq!(
            self.parents.len(),
            self.stop_count * MAX_ROUNDS + 1,
            "there is a parent slot for each stop in each round, and a spare"
        );
        for (stop, &marked) in self.marked_stops.iter().enumerate() {
            assert!(
                !marked || !self.tau_star[stop].is_empty(),
                "a stop is only marked when it was given a label"
            );
        }
        self.target.labels.check_invariants();
        assert_eq!(
            self.target.best_stop.is_some(),
            self.target.best_round.is_some(),
            "the best target stop and round are recorded together"
        );
        assert!(
            self.target
                .best_round
                .is_none_or(|round| round < MAX_ROUNDS),
            "the best round is a round the search runs"
        );
    }

    fn parent_slot(&self, round: usize, stop: StopIndex) -> usize {
        round * self.stop_count + stop.as_usize()
    }
}

/// The labels that reach the destination, and where the cheapest of them came from.
#[derive(Debug)]
struct Target {
    labels: ParetoFront,
    /// Stop that led to the cheapest label.
    best_stop: Optional<StopIndex>,
    /// Round that led to the cheapest label.
    best_round: Option<usize>,
}

impl Target {
    fn new() -> Self {
        Self {
            labels: ParetoFront::new(),
            best_stop: Optional::new(StopIndex::NONE),
            best_round: None,
        }
    }

    fn clear(&mut self) {
        self.labels.clear();
        self.best_stop = Optional::new(StopIndex::NONE);
        self.best_round = None;
    }

    /// Adds a label that reaches the target from `stop`, and remembers where it came from if it is
    /// now the cheapest.
    fn offer(&mut self, stop: StopIndex, round: usize, label: ParetoLabel, is_arrival: bool) {
        if self.labels.add(label, is_arrival) && label.cost <= self.labels.best_cost() {
            self.best_stop = Optional::new(stop);
            self.best_round = Some(round);
        }
    }
}

/// Starts the scan of each pattern through `stop` from the stop, where that is earlier going
/// forward or later going back than where it starts now.
fn activate_patterns_at(
    active: &mut [Optional<PatternPosition>],
    consumer: &Consumer<'_>,
    stop: StopIndex,
    is_arrival: bool,
) {
    for pattern in consumer.iter_trip_patterns_by_stop(stop) {
        let stops = consumer.stop_sequence_by_trip_pattern(pattern.index);
        if let Some(position) = stops.iter().position(|&other| other == stop) {
            let position = PatternPosition::from(position);
            activate(active, pattern.index, position, is_arrival);
        }
    }
}

/// Moves the start of a pattern's scan to `position` if that is earlier going forward, or later
/// going back.
fn activate(
    active: &mut [Optional<PatternPosition>],
    pattern: TripPatternIndex,
    position: PatternPosition,
    is_arrival: bool,
) {
    let slot = &mut active[pattern.as_usize()];
    let better = slot.as_option().is_none_or(|current| {
        if is_arrival {
            position > current
        } else {
            position < current
        }
    });
    if better {
        *slot = Optional::new(position);
    }
}

#[cfg(test)]
mod fuzz;
