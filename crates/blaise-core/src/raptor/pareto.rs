use flatnet_core::models::{Duration, Time};

/// Narrows a cost summed in `f64` to the `f32` a label stores.
#[allow(
    clippy::cast_possible_truncation,
    reason = "a cost is a few thousand weighted seconds, far inside f32's range"
)]
#[must_use]
pub(crate) fn narrow_cost(cost: f64) -> f32 {
    cost as f32
}

/// Where a label sits in the search's record of how stops were reached.
pub(crate) type NodeId = u32;

/// The node of a label that has not been recorded, such as one built to compare against.
pub(crate) const NO_NODE: NodeId = NodeId::MAX;

/// A time at a stop and what it cost to get there, and the node that says how it was reached.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(crate) struct ParetoLabel {
    pub time: Time,
    pub cost: f32,
    pub node: NodeId,
}

impl ParetoLabel {
    #[must_use]
    pub(crate) const fn new(time: Time, cost: f32) -> Self {
        Self {
            time,
            cost,
            node: NO_NODE,
        }
    }

    /// The same label, reached the way `node` says.
    #[must_use]
    pub(crate) const fn with_node(self, node: NodeId) -> Self {
        Self { node, ..self }
    }

    /// The label after `walk` more of walking, which costs `penalty` for every second: later going
    /// forward, earlier when searching back from an arrival.
    #[must_use]
    pub(crate) fn after_walk(self, walk: Duration, penalty: f64, is_arrival: bool) -> Self {
        let time = if is_arrival {
            self.time - walk
        } else {
            self.time + walk
        };
        let cost = f64::from(walk.0).mul_add(penalty, f64::from(self.cost));
        Self::new(time, narrow_cost(cost)).with_node(self.node)
    }
}

/// Whether a label at `label_time` is at least as good as one at `time`: earlier when searching
/// forward from a departure, later when searching back from an arrival.
fn at_least_as_good(label_time: Time, time: Time, is_arrival: bool) -> bool {
    if is_arrival {
        label_time >= time
    } else {
        label_time <= time
    }
}

/// Panics in debug builds if one of two labels on a front beats the other, or if they sit on
/// different sides in time than the pairs before them.
fn check_pair(first: ParetoLabel, second: ParetoLabel, cheaper_is_later: &mut Option<bool>) {
    assert!(
        first.cost.total_cmp(&second.cost).is_ne(),
        "no two labels on a front cost the same, or the later one would be beaten"
    );
    let (cheaper, dearer) = if first.cost < second.cost {
        (first, second)
    } else {
        (second, first)
    };
    assert!(
        cheaper.time != dearer.time,
        "two labels at one time differ in cost, so the dearer would be beaten"
    );
    let later = cheaper.time > dearer.time;
    assert_eq!(
        *cheaper_is_later.get_or_insert(later),
        later,
        "the cheaper label is on the same side in time for every pair"
    );
}

/// The labels at a stop that no other label beats in both time and cost.
///
/// After every public call no two labels cost the same, and the cheaper of any two is the later
/// one when searching forward and the earlier one when searching back, the same for every pair.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParetoFront(Vec<ParetoLabel>);

impl ParetoFront {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if `(time, cost)` is dominated by any label already on the front.
    #[must_use]
    pub(crate) fn is_dominated(&self, time: Time, cost: f32, is_arrival: bool) -> bool {
        self.0
            .iter()
            .any(|label| at_least_as_good(label.time, time, is_arrival) && label.cost <= cost)
    }

    /// Adds a label if it is not dominated, and removes the labels it dominates. Returns `true`
    /// if the label was accepted.
    pub(crate) fn add(&mut self, label: ParetoLabel, is_arrival: bool) -> bool {
        if self.is_dominated(label.time, label.cost, is_arrival) {
            return false;
        }
        self.0.retain(|other| {
            !(at_least_as_good(label.time, other.time, is_arrival) && label.cost <= other.cost)
        });
        self.0.push(label);
        self.check_invariants();
        true
    }

    /// Panics in debug builds if one label beats another.
    pub(crate) fn check_invariants(&self) {
        if !cfg!(debug_assertions) {
            return;
        }
        let mut cheaper_is_later = None;
        for (index, first) in self.0.iter().enumerate() {
            for second in &self.0[index + 1..] {
                check_pair(*first, *second, &mut cheaper_is_later);
            }
        }
    }

    /// Minimum cost across all labels, `f32::MAX` if empty.
    #[must_use]
    pub(crate) fn best_cost(&self) -> f32 {
        self.0
            .iter()
            .fold(f32::MAX, |best, label| best.min(label.cost))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = ParetoLabel> + '_ {
        self.0.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(time: u32, cost: u32) -> ParetoLabel {
        #[allow(clippy::cast_precision_loss, reason = "the cost is tiny")]
        ParetoLabel::new(Time(time), cost as f32)
    }

    #[test]
    fn an_equal_label_is_beaten_and_not_added_twice() {
        let mut front = ParetoFront::new();

        assert!(front.add(label(10, 5), false), "the first label is kept");
        assert!(
            !front.add(label(10, 5), false),
            "an equal one is beaten by it"
        );
    }

    #[test]
    fn going_forward_an_earlier_dearer_label_and_a_later_cheaper_one_coexist() {
        let mut front = ParetoFront::new();
        front.add(label(10, 9), false);

        assert!(
            front.add(label(20, 3), false),
            "later but cheaper beats nothing"
        );
        assert!(
            front.add(label(5, 12), false),
            "earlier but dearer beats nothing"
        );
        assert_eq!(front.best_cost(), 3.0, "the cheapest is the later one");
    }

    #[test]
    fn a_label_that_beats_others_removes_them() {
        let mut front = ParetoFront::new();
        front.add(label(10, 9), false);
        front.add(label(20, 3), false);

        let added = front.add(label(10, 3), false);

        assert!(added, "earlier and as cheap beats both");
        assert_eq!(front.iter().count(), 1, "the two it beats are gone");
    }

    #[test]
    fn going_back_a_later_label_beats_an_earlier_one() {
        let mut front = ParetoFront::new();
        front.add(label(10, 5), true);

        assert!(
            front.is_dominated(Time(8), 5.0, true),
            "a later departure is better when searching back"
        );
        assert!(
            !front.is_dominated(Time(12), 5.0, true),
            "an earlier one is not beaten by it"
        );
    }

    #[test]
    fn walking_moves_the_time_the_way_the_search_runs_and_adds_to_the_cost() {
        let start = label(100, 10);

        let forward = start.after_walk(Duration(30), 2.0, false);
        let back = start.after_walk(Duration(30), 2.0, true);

        assert_eq!(forward.time, Time(130), "forward, the walk ends later");
        assert_eq!(back.time, Time(70), "back, the walk starts earlier");
        assert_eq!(forward.cost, 70.0, "30 s at 2 per second on top of 10");
    }

    #[test]
    fn a_front_with_no_labels_costs_the_most() {
        assert_eq!(
            ParetoFront::new().best_cost(),
            f32::MAX,
            "an empty front costs the most"
        );
    }
}

#[cfg(test)]
mod fuzz;
