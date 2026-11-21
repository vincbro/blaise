use crate::raptor::itinerary::Itinerary;

/// The journeys a search found, none of which another beats on every measure. Blaise does not
/// say which is best: read the measures of each [`Itinerary`] and pick, or use one of the
/// selectors for the common choices.
#[derive(Debug, Clone)]
pub struct Journeys {
    pub itineraries: Box<[Itinerary]>,
}

impl Journeys {
    pub(crate) fn new(itineraries: Vec<Itinerary>) -> Self {
        Self {
            itineraries: itineraries.into_boxed_slice(),
        }
    }

    /// The journey that arrives first. Of two that arrive together, the one with fewer changes.
    #[must_use]
    pub fn earliest_arrival(&self) -> Option<&Itinerary> {
        self.itineraries.iter().min_by_key(|itinerary| {
            (
                itinerary.arrival().map(|time| time.actual),
                itinerary.rides(),
            )
        })
    }

    /// The journey that takes the least time on the way. Of two that take as long, the one with
    /// fewer changes.
    #[must_use]
    pub fn fastest(&self) -> Option<&Itinerary> {
        self.itineraries
            .iter()
            .min_by_key(|itinerary| (itinerary.duration().0, itinerary.rides()))
    }

    /// The journey that boards the fewest vehicles. Of two with as many, the one that arrives
    /// first.
    #[must_use]
    pub fn fewest_changes(&self) -> Option<&Itinerary> {
        self.itineraries.iter().min_by_key(|itinerary| {
            (
                itinerary.rides(),
                itinerary.arrival().map(|time| time.actual),
            )
        })
    }
}
