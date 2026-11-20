use gtfs_bin::models::{StopIndex, Time, TripIndex};

use crate::raptor::{LiveTime, PatternPosition};

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

/// A label found for a stop in one round, with how it was reached.
#[derive(Debug)]
pub(crate) struct Update {
    pub stop: StopIndex,
    pub time: Time,
    pub cost: f32,
    pub parent: Parent,
}

impl Update {
    #[must_use]
    pub(crate) const fn new(stop: StopIndex, time: Time, cost: f32, parent: Parent) -> Self {
        Self {
            stop,
            time,
            cost,
            parent,
        }
    }
}
