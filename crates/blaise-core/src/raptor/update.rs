use flatnet_core::models::{StopIndex, Time, TripIndex};

use crate::raptor::{Day, LiveTime, PatternPosition, pareto::NodeId};

/// How a passenger moves between two stops without a vehicle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepKind {
    Transfer,
    Walk,
}

/// How a stop was reached.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Parent {
    Transit {
        boarding_position: PatternPosition,
        alighting_position: PatternPosition,
        trip: TripIndex,
        day: Day,
        departure_time: LiveTime,
        arrival_time: LiveTime,
    },
    Transfer {
        from_stop: StopIndex,
        departure_time: LiveTime,
        arrival_time: LiveTime,
    },
    Walk {
        from_stop: StopIndex,
        departure_time: LiveTime,
        arrival_time: LiveTime,
    },
    Origin,
}

impl Parent {
    /// A move between stops on foot or by a listed transfer.
    #[must_use]
    pub(crate) const fn step(
        kind: StepKind,
        from_stop: StopIndex,
        departure_time: LiveTime,
        arrival_time: LiveTime,
    ) -> Self {
        match kind {
            StepKind::Transfer => Self::Transfer {
                from_stop,
                departure_time,
                arrival_time,
            },
            StepKind::Walk => Self::Walk {
                from_stop,
                departure_time,
                arrival_time,
            },
        }
    }
}

/// One step of the way to a label: the stop it is at, when, how it got there, and the node of
/// the label it came from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Node {
    pub stop: StopIndex,
    pub time: Time,
    pub parent: Parent,
    pub previous: NodeId,
}

/// A label found for a stop in one round, with how it was reached and from which label.
#[derive(Debug)]
pub(crate) struct Update {
    pub stop: StopIndex,
    pub time: Time,
    pub cost: f32,
    pub parent: Parent,
    pub previous: NodeId,
}

impl Update {
    #[must_use]
    pub(crate) const fn new(
        stop: StopIndex,
        time: Time,
        cost: f32,
        parent: Parent,
        previous: NodeId,
    ) -> Self {
        Self {
            stop,
            time,
            cost,
            parent,
            previous,
        }
    }
}
