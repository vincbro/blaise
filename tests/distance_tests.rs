use ontrack::engine::geo::{Coordinate, Distance};

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
    let d = coord_a.euclidean_distance(&coord_b);
    assert!((d.as_kilometers() - 343_000.0).abs() > 500.0);
}

#[test]
fn distance_eq_test() {
    let dist_a = Distance::meters(1000.0);
    let dist_b = Distance::kilometers(1.0);
    assert_eq!(dist_a, dist_b)
}

#[test]
fn distance_cmp_test() {
    let dist_a = Distance::meters(1000.0);
    let dist_b = Distance::kilometers(0.5);
    assert!(dist_a > dist_b)
}
