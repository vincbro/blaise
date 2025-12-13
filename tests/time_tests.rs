use ontrack::engine::parse_gtfs_time;

#[test]
fn valid_time_test_1() {
    let time = "00:00:00";
    assert_eq!(parse_gtfs_time(time).unwrap(), 0);
}

#[test]
fn valid_time_test_2() {
    let time = "00:00:30";
    assert_eq!(parse_gtfs_time(time).unwrap(), 30);
}

#[test]
fn valid_time_test_3() {
    let time = "00:01:30";
    assert_eq!(parse_gtfs_time(time).unwrap(), 90);
}

#[test]
fn valid_time_test_4() {
    let time = "01:01:30";
    assert_eq!(parse_gtfs_time(time).unwrap(), 3690);
}

#[test]
fn invalid_time_test_1() {
    let time = "00:00:0a";
    assert!(parse_gtfs_time(time).is_none())
}
#[test]
fn invalid_time_test_2() {
    let time = "00:00";
    assert!(parse_gtfs_time(time).is_none())
}
