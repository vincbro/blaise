use gtfs_bin_core::models::Time;
use proptest::{collection::vec, prelude::*};

use super::{ParetoFront, ParetoLabel};

/// A time and a cost, small ranges so that ties and near-ties come up often.
pub(crate) fn label() -> impl Strategy<Value = ParetoLabel> {
    (0_u32..50, 0_u32..50).prop_map(|(time, cost)| {
        #[allow(clippy::cast_precision_loss, reason = "the cost is below 50")]
        ParetoLabel::new(Time(time), cost as f32)
    })
}

proptest! {
    /// The front answers like a plain list of every label it accepted, and keeps its invariants,
    /// which `add` checks, after every call.
    #[test]
    fn front_matches_a_list_of_accepted_labels(
        is_arrival in any::<bool>(),
        labels in vec(label(), 0..60),
        probes in vec(label(), 0..20),
    ) {
        let mut front = ParetoFront::new();
        let mut accepted: Vec<ParetoLabel> = Vec::new();
        for label in labels {
            let was_dominated = front.is_dominated(label.time, label.cost, is_arrival);
            let added = front.add(label, is_arrival);
            prop_assert_eq!(added, !was_dominated, "a label is added exactly when nothing beats it");
            if added {
                accepted.push(label);
            }
        }

        for probe in probes {
            let beaten = accepted.iter().any(|label| {
                let time_ok = if is_arrival { label.time >= probe.time } else { label.time <= probe.time };
                time_ok && label.cost <= probe.cost
            });
            prop_assert_eq!(
                front.is_dominated(probe.time, probe.cost, is_arrival),
                beaten,
                "a probe is beaten when any accepted label beats it, pruned or not"
            );
        }

        let cheapest = accepted.iter().map(|label| label.cost).fold(f32::MAX, f32::min);
        prop_assert!(
            front.best_cost().total_cmp(&cheapest).is_eq(),
            "pruning never drops the cheapest label"
        );
    }
}
