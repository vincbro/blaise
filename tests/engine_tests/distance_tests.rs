use ontrack::engine::{Coordinate, distance};

#[test]
fn distance_test() {
    let coord_a = Coordinate {
        latitude: 48.85800943005911,
        longitude: 2.3514350059357927,
    };

    let coord_b = Coordinate {
        latitude: 51.5052389927712,
        longitude: -0.12495407345099824,
    };
    let d = distance(&coord_a, &coord_b);
    assert!((d - 343_000.0).abs() > 500.0);
}
