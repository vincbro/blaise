use flatnet_core::models::{Duration, StopIndex, Time};
use proptest::{collection::vec, prelude::*};

use super::State;
use crate::{
    raptor::{
        MAX_ROUNDS,
        pareto::ParetoLabel,
        update::{Parent, Update},
    },
    spatial::fuzz::position,
    test_support,
};

/// One call on a `State`, with stops given as a fraction of the network's stops.
#[derive(Debug, Clone)]
enum Operation {
    Seed {
        stop: f64,
        time: u32,
        cost: u8,
    },
    AddTarget {
        stop: f64,
        seconds: u32,
    },
    Apply {
        round: usize,
        updates: Vec<(f64, u32, u8)>,
    },
    BeginRound,
    RecordTargets {
        round: usize,
    },
    Reset,
}

fn operation() -> impl Strategy<Value = Operation> {
    let stop = 0.0_f64..1.0;
    let update = (stop.clone(), 0_u32..100, 0_u8..50);
    prop_oneof![
        (stop.clone(), 0_u32..100, 0_u8..50).prop_map(|(stop, time, cost)| Operation::Seed {
            stop,
            time,
            cost
        }),
        (stop.clone(), 0_u32..100)
            .prop_map(|(stop, seconds)| Operation::AddTarget { stop, seconds }),
        (0..MAX_ROUNDS, vec(update, 0..12))
            .prop_map(|(round, updates)| Operation::Apply { round, updates }),
        Just(Operation::BeginRound),
        (0..MAX_ROUNDS).prop_map(|round| Operation::RecordTargets { round }),
        Just(Operation::Reset),
    ]
}

/// The stop a fraction of the network's stops points at.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the fraction is below 1 and the network has a few dozen stops"
)]
fn stop_at(fraction: f64, stop_count: usize) -> StopIndex {
    StopIndex::from(((fraction * stop_count as f64) as usize).min(stop_count - 1))
}

proptest! {
    // Each case compiles a network, which is slow, so it runs a quarter of the configured cases.
    #![proptest_config(ProptestConfig { cases: ProptestConfig::default().cases / 4, ..ProptestConfig::default() })]

    /// Whatever the calls and their order, the state keeps its invariants, which every call
    /// checks, and a stop that was offered a label is afterwards at least as good as it.
    #[test]
    fn state_survives_any_sequence_of_calls(
        is_arrival in any::<bool>(),
        positions in vec(position(), 2..12),
        operations in vec(operation(), 0..40),
    ) {
        let visits: Vec<usize> = (0..positions.len()).collect();
        let network = test_support::network(&positions, &[visits.clone(), visits.into_iter().rev().collect()]);
        let consumer = network.consumer();
        let stop_count = positions.len();
        let mut state = State::new(&consumer);

        for operation in operations {
            match operation {
                Operation::Seed { stop, time, cost } => {
                    let label = ParetoLabel::new(Time(time), f32::from(cost));
                    state.seed(stop_at(stop, stop_count), label, is_arrival);
                }
                Operation::AddTarget { stop, seconds } => {
                    state.add_target(stop_at(stop, stop_count), Duration(seconds));
                }
                Operation::Apply { round, updates } => {
                    let updates: Vec<Update> = updates
                        .into_iter()
                        .map(|(stop, time, cost)| {
                            Update::new(stop_at(stop, stop_count), Time(time), f32::from(cost), Parent::Origin)
                        })
                        .collect();
                    state.apply_updates(round, is_arrival, &updates);
                    for update in &updates {
                        prop_assert!(
                            state.is_dominated(update.stop, update.time, update.cost, is_arrival),
                            "an offered label is kept or beaten by one that is"
                        );
                    }
                }
                Operation::BeginRound => state.begin_round(&consumer, is_arrival),
                Operation::RecordTargets { round } => state.record_targets(round, is_arrival, 2.0),
                Operation::Reset => state.reset(),
            }
        }
    }
}
