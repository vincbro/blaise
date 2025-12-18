use std::{
    cmp,
    fmt::Display,
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
};

use serde::{Deserialize, Serialize};

pub(crate) const AVERAGE_STOP_DISTANCE: Distance = Distance::from_meters(500.0);
pub(crate) const LONGITUDE_DISTANCE: Distance = Distance::from_meters(111_320.0);
pub(crate) const LATITUDE_DISTANCE: Distance = Distance::from_meters(110_540.0);

#[derive(Debug, Clone, Copy, Default)]
pub struct Distance(f64);

impl PartialEq for Distance {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Add for Distance {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Distance {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul for Distance {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Div for Distance {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

impl Distance {
    pub const fn from_meters(distance: f64) -> Self {
        Self(distance)
    }

    pub const fn from_kilometers(distance: f64) -> Self {
        Self(distance * 1000.0)
    }

    pub const fn as_meters(&self) -> f64 {
        self.0
    }

    pub const fn as_kilometers(&self) -> f64 {
        self.0 / 1000.0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

impl Display for Coordinate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}, {}", self.latitude, self.longitude))
    }
}

impl Sum for Coordinate {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut count: usize = 0;
        let mut lat: f64 = 0.0;
        let mut lon: f64 = 0.0;
        iter.for_each(|coordinate| {
            count += 1;
            lat += coordinate.latitude;
            lon += coordinate.longitude;
        });
        let count = count as f64;
        Self {
            latitude: lat / count,
            longitude: lon / count,
        }
    }
}

impl From<Coordinate> for (f64, f64) {
    fn from(value: Coordinate) -> Self {
        (value.latitude, value.longitude)
    }
}

impl Coordinate {
    pub fn euclidean_distance(&self, coord: &Self) -> Distance {
        const R: f64 = 6371.0;
        let dist_lat = f64::to_radians(coord.latitude - self.latitude);
        let dist_lon = f64::to_radians(coord.longitude - self.longitude);
        let a = f64::powi(f64::sin(dist_lat / 2.0), 2)
            + f64::cos(f64::to_radians(self.latitude))
                * f64::cos(f64::to_radians(coord.latitude))
                * f64::sin(dist_lon / 2.0)
                * f64::sin(dist_lon / 2.0);
        let c = 2.0 * f64::atan2(f64::sqrt(a), f64::sqrt(1.0 - a));
        Distance::from_kilometers(R * c)
    }

    pub fn network_distance(&self, coord: &Self) -> Distance {
        const CIRCUITY_FACTOR: f64 = 1.3;
        Distance::from_meters(self.euclidean_distance(coord).as_meters() * CIRCUITY_FACTOR)
    }

    pub fn to_grid(&self) -> (i32, i32) {
        let x = (self.longitude * LONGITUDE_DISTANCE.as_meters()
            / AVERAGE_STOP_DISTANCE.as_meters()) as i32;
        let y = (self.latitude * LATITUDE_DISTANCE.as_meters() / AVERAGE_STOP_DISTANCE.as_meters())
            as i32;
        (x, y)
    }
}

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
    let dist_a = Distance::from_meters(1000.0);
    let dist_b = Distance::from_kilometers(1.0);
    assert_eq!(dist_a, dist_b)
}

#[test]
fn distance_cmp_test() {
    let dist_a = Distance::from_meters(1000.0);
    let dist_b = Distance::from_kilometers(0.5);
    assert!(dist_a > dist_b)
}
