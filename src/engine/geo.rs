use std::{
    cmp,
    fmt::Display,
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
};

use serde::{Deserialize, Serialize};

use crate::engine::{AVERAGE_STOP_DISTANCE, LATITUDE_DISTANCE, LONGITUDE_DISTANCE};

#[derive(Debug, Clone, Copy)]
pub enum Distance {
    Meter(f64),
    Kilometers(f64),
}

impl Default for Distance {
    fn default() -> Self {
        Self::Meter(0.0)
    }
}

impl PartialEq for Distance {
    fn eq(&self, other: &Self) -> bool {
        self.as_meters() == other.as_meters()
    }
}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_meters().partial_cmp(&other.as_meters())
    }
}

impl Add for Distance {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::meters(self.as_meters() + rhs.as_meters())
    }
}

impl Sub for Distance {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::meters(self.as_meters() - rhs.as_meters())
    }
}

impl Mul for Distance {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self::meters(self.as_meters() * rhs.as_meters())
    }
}

impl Div for Distance {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::meters(self.as_meters() / rhs.as_meters())
    }
}

impl Distance {
    pub const fn meters(distance: f64) -> Self {
        Self::Meter(distance)
    }

    pub const fn kilometers(distance: f64) -> Self {
        Self::Kilometers(distance)
    }

    pub const fn as_meters(&self) -> f64 {
        match self {
            Distance::Meter(value) => *value,
            Distance::Kilometers(value) => *value * 1000.0,
        }
    }

    pub const fn as_kilometers(&self) -> f64 {
        match self {
            Distance::Meter(value) => *value * 0.001,
            Distance::Kilometers(value) => *value,
        }
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
    pub fn distance(&self, coord: &Self) -> Distance {
        const R: f64 = 6371.0;
        let dist_lat = f64::to_radians(coord.latitude - self.latitude);
        let dist_lon = f64::to_radians(coord.longitude - self.longitude);
        let a = f64::powi(f64::sin(dist_lat / 2.0), 2)
            + f64::cos(f64::to_radians(self.latitude))
                * f64::cos(f64::to_radians(coord.latitude))
                * f64::sin(dist_lon / 2.0)
                * f64::sin(dist_lon / 2.0);
        let c = 2.0 * f64::atan2(f64::sqrt(a), f64::sqrt(1.0 - a));
        Distance::kilometers(R * c)
    }

    pub fn to_grid(&self) -> (i32, i32) {
        let x = (self.longitude * LONGITUDE_DISTANCE.as_meters()
            / AVERAGE_STOP_DISTANCE.as_meters()) as i32;
        let y = (self.latitude * LATITUDE_DISTANCE.as_meters() / AVERAGE_STOP_DISTANCE.as_meters())
            as i32;
        (x, y)
    }
}
