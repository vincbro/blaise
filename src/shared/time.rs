use std::{
    ops::{Add, AddAssign, Sub, SubAssign},
    sync::Arc,
    time::Instant,
};

use chrono::{Local, Timelike};
use zip::extra_fields::ExtendedTimestamp;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time(u32);

impl From<u32> for Time {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Sub<Time> for Time {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        Duration(self.0 - rhs.0)
    }
}

impl Add<Time> for Time {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<Time> for Time {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Add<Duration> for Time {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<Duration> for Time {
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs.0
    }
}

impl Time {
    pub fn now() -> Self {
        let now = Local::now();
        Self(now.num_seconds_from_midnight())
    }

    pub const fn from_seconds(secs: u32) -> Self {
        Self(secs)
    }

    pub const fn as_seconds(&self) -> u32 {
        self.0
    }

    pub fn to_hms_string(&self) -> String {
        let h = self.0 / 3600;
        let m = (self.0 % 3600) / 60;
        let s = self.0 % 60;
        format!("{:02}:{:02}:{:02}", h, m, s)
    }

    pub fn from_hms(time: &str) -> Option<Self> {
        const HOUR_TO_SEC: u32 = 60 * 60;
        const MINUTE_TO_SEC: u32 = 60;
        let mut split = time.split(':');
        let hours: u32 = split.next()?.parse().ok()?;
        let hours = hours * HOUR_TO_SEC;
        let minutes: u32 = split.next()?.parse().ok()?;
        let minutes = minutes * MINUTE_TO_SEC;
        let seconds: u32 = split.next()?.parse().ok()?;
        let seconds = hours + minutes + seconds;
        Some(Self(seconds))
    }
}

#[test]
fn parse_unparse_1() {
    let time = "00:00:00";
    let stime = Time::from_hms(time).unwrap();
    assert_eq!(time, stime.to_hms_string())
}

#[test]
fn parse_unparse_2() {
    let time = "00:00:30";
    let stime = Time::from_hms(time).unwrap();
    assert_eq!(time, stime.to_hms_string())
}

#[test]
fn parse_unparse_3() {
    let time = "00:30:00";
    let stime = Time::from_hms(time).unwrap();
    assert_eq!(time, stime.to_hms_string())
}

#[test]
fn parse_unparse_4() {
    let time = "12:00:00";
    let stime = Time::from_hms(time).unwrap();
    assert_eq!(time, stime.to_hms_string())
}

#[test]
fn parse_unparse_5() {
    let time = "12:30:30";
    let stime = Time::from_hms(time).unwrap();
    assert_eq!(time, stime.to_hms_string())
}
#[test]
fn valid_time_test_1() {
    let time = "00:00:00";
    assert_eq!(Time::from_hms(time).unwrap().as_seconds(), 0);
}

#[test]
fn valid_time_test_2() {
    let time = "00:00:30";
    assert_eq!(Time::from_hms(time).unwrap().as_seconds(), 30);
}

#[test]
fn valid_time_test_3() {
    let time = "00:01:30";
    assert_eq!(Time::from_hms(time).unwrap().as_seconds(), 90);
}

#[test]
fn valid_time_test_4() {
    let time = "01:01:30";
    assert_eq!(Time::from_hms(time).unwrap().as_seconds(), 3690);
}

#[test]
fn invalid_time_test_1() {
    let time = "00:00:0a";
    assert!(Time::from_hms(time).is_none())
}
#[test]
fn invalid_time_test_2() {
    let time = "00:00";
    assert!(Time::from_hms(time).is_none())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(u32);

impl From<u32> for Duration {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Duration {
    pub const fn from_seconds(secs: u32) -> Self {
        Self(secs)
    }

    pub const fn from_minutes(minutes: u32) -> Self {
        Self(minutes * 60)
    }

    pub const fn from_hours(hours: u32) -> Self {
        Self(hours * 60 * 60)
    }

    pub const fn from_days(days: u32) -> Self {
        Self(days * 60 * 60 * 24)
    }

    pub const fn as_seconds(&self) -> u32 {
        self.0
    }
}

impl Sub for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0
    }
}

impl Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}
