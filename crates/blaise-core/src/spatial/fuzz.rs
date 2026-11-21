use std::collections::HashSet;

use flatnet_core::models::Coordinate;
use proptest::{collection::vec, prelude::*};

use super::SpatialGrid;
use crate::test_support;

/// A position in a box about 55 km by 45 km, so a planar grid is a fair stand-in for the sphere.
pub(crate) fn position() -> impl Strategy<Value = (f64, f64)> {
    (59.0_f64..59.5, 18.0_f64..18.8)
}

proptest! {
    // Each case compiles a network, which is slow, so it runs a quarter of the configured cases.
    #![proptest_config(ProptestConfig { cases: ProptestConfig::default().cases / 4, ..ProptestConfig::default() })]

    /// The grid finds the stops that lie within the radius, each once, and none far outside it.
    /// The margins are for the grid measuring on a flat plane and the model on the sphere.
    /// `new` checks the grid's invariants.
    #[test]
    fn radius_search_matches_brute_force(
        positions in vec(position(), 2..40),
        probes in vec(position(), 1..8),
        radius in 0.0_f64..3000.0,
    ) {
        let network = test_support::network(&positions, &[vec![0, 1]]);
        let consumer = network.consumer();
        let grid = SpatialGrid::new(&consumer);

        for (latitude, longitude) in probes {
            let probe = Coordinate::new(latitude, longitude);
            let found: Vec<_> = grid.iter_stops_in_radius(probe, radius).collect();
            let unique: HashSet<_> = found.iter().copied().collect();
            prop_assert_eq!(unique.len(), found.len(), "a stop is found once");

            for stop in consumer.stops() {
                let distance = probe.distance_meters(stop.coordinate.as_option().expect("a position"));
                let is_found = unique.contains(&stop.index);
                prop_assert!(
                    distance >= radius * 0.98 || is_found,
                    "a stop well inside the radius is found: {distance} m of {radius} m"
                );
                prop_assert!(
                    distance <= radius * 1.02 + 1.0 || !is_found,
                    "a stop well outside the radius is not found: {distance} m of {radius} m"
                );
            }
        }
    }
}
