use crate::repository::Cell;
use serde::{Deserialize, Serialize};
use std::{
    cmp,
    fmt::Display,
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
    str::FromStr,
};
use thiserror::Error;

pub(crate) const AVERAGE_STOP_DISTANCE: Distance = Distance::from_meters(500.0);
pub(crate) const LONGITUDE_DISTANCE: Distance = Distance::from_meters(111_320.0);
pub(crate) const LATITUDE_DISTANCE: Distance = Distance::from_meters(110_540.0);

#[derive(Debug, Clone, Copy, Default)]
pub struct Distance(f32);

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

impl From<f32> for Distance {
    fn from(value: f32) -> Self {
        Distance(value)
    }
}

impl Distance {
    pub const fn from_meters(distance: f32) -> Self {
        Self(distance)
    }

    pub const fn from_kilometers(distance: f32) -> Self {
        Self(distance * 1000.0)
    }

    pub const fn as_meters(&self) -> f32 {
        self.0
    }

    pub const fn as_kilometers(&self) -> f32 {
        self.0 / 1000.0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub latitude: f32,
    pub longitude: f32,
}

impl Sum for Coordinate {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut count: usize = 0;
        let mut lat: f32 = 0.0;
        let mut lon: f32 = 0.0;
        iter.for_each(|coordinate| {
            count += 1;
            lat += coordinate.latitude;
            lon += coordinate.longitude;
        });
        let count = count as f32;
        Self {
            latitude: lat / count,
            longitude: lon / count,
        }
    }
}

impl From<Coordinate> for (f32, f32) {
    fn from(value: Coordinate) -> Self {
        (value.latitude, value.longitude)
    }
}

impl Display for Coordinate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}, {}", self.latitude, self.longitude))
    }
}

#[derive(Error, Debug)]
pub enum ParseCoordinateError {
    #[error("Invalid latitude")]
    InvalidLatitude,
    #[error("Invalid longitude")]
    InvalidLongitude,
    #[error("Invalid format")]
    InvalidFormat,
}

impl FromStr for Coordinate {
    type Err = ParseCoordinateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.contains(',') {
            return Err(ParseCoordinateError::InvalidFormat);
        }
        let s: String = s.split_whitespace().collect();
        let split: Vec<_> = s.split(',').collect();
        let latitude: f32 = split
            .first()
            .ok_or(ParseCoordinateError::InvalidLatitude)?
            .parse()
            .map_err(|_| ParseCoordinateError::InvalidLatitude)?;
        let longitude: f32 = split
            .last()
            .ok_or(ParseCoordinateError::InvalidLongitude)?
            .parse()
            .map_err(|_| ParseCoordinateError::InvalidLongitude)?;
        Ok(Coordinate {
            latitude,
            longitude,
        })
    }
}

impl Coordinate {
    pub fn euclidean_distance(&self, coord: &Self) -> Distance {
        const R: f32 = 6371.0;
        let dist_lat = f32::to_radians(coord.latitude - self.latitude);
        let dist_lon = f32::to_radians(coord.longitude - self.longitude);
        let a = f32::powi(f32::sin(dist_lat / 2.0), 2)
            + f32::cos(f32::to_radians(self.latitude))
                * f32::cos(f32::to_radians(coord.latitude))
                * f32::sin(dist_lon / 2.0)
                * f32::sin(dist_lon / 2.0);
        let c = 2.0 * f32::atan2(f32::sqrt(a), f32::sqrt(1.0 - a));
        Distance::from_kilometers(R * c)
    }

    pub fn network_distance(&self, coord: &Self) -> Distance {
        const CIRCUITY_FACTOR: f32 = 1.3;
        Distance::from_meters(self.euclidean_distance(coord).as_meters() * CIRCUITY_FACTOR)
    }

    pub fn to_cell(&self) -> Cell {
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
        latitude: 48.858_01,
        longitude: 2.351_435,
    };

    let coord_b = Coordinate {
        latitude: 51.505_238,
        longitude: -0.124_954_075,
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
