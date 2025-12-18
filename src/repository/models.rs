use std::sync::Arc;

use crate::shared::{
    Identifiable,
    geo::{Coordinate, Distance},
    time::{Duration, Time},
};

#[derive(Debug, Default, Clone)]
pub struct Area {
    pub index: u32,
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub normalized_name: Arc<str>,
}

impl Identifiable for Area {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn normalized_name(&self) -> &str {
        &self.normalized_name
    }
}

#[derive(Debug, Default, Clone)]
pub enum LocationType {
    #[default]
    Stop,
    Platform {
        parent_station: Arc<str>,
        platform_code: Arc<str>,
    },
    Station,
    Entrance(Arc<str>),
    Node,
    Boarding,
}

#[derive(Debug, Default, Clone)]
pub struct Stop {
    pub index: u32,
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub normalized_name: Arc<str>,
    pub coordinate: Coordinate,
    pub location_type: LocationType,
}

impl Identifiable for Stop {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn normalized_name(&self) -> &str {
        &self.normalized_name
    }
}

#[derive(Debug, Default, Clone)]
pub enum Timepoint {
    #[default]
    Approximate,
    Exact,
}

#[derive(Debug, Default, Clone)]
pub enum StopAccessType {
    #[default]
    Regularly,
    NoneAvailable,
    AgencyArrange,
    DriverArrange,
}

#[derive(Debug, Default, Clone)]
pub struct StopTime {
    pub trip_idx: u32,
    pub trip_id: Arc<str>,
    pub stop_idx: u32,
    pub stop_id: Arc<str>,
    pub sequence: u16,
    // Seconds since midnight
    pub arrival_time: Time,
    // Seconds since midnight
    pub departure_time: Time,
    pub headsign: Option<Arc<str>>,
    pub dist_traveled: Option<Distance>,
    pub pickup_type: StopAccessType,
    pub drop_off_type: StopAccessType,
    pub timepoint: Timepoint,
}

#[derive(Debug, Default, Clone)]
pub struct Transfer {
    pub from_stop_id: Arc<str>,
    pub from_stop_idx: u32,
    pub to_stop_id: Arc<str>,
    pub to_stop_idx: u32,

    pub from_trip_id: Option<Arc<str>>,
    pub from_trip_idx: Option<u32>,
    pub to_trip_id: Option<Arc<str>>,
    pub to_trip_idx: Option<u32>,

    pub min_transfer_time: Option<Duration>,
}

#[derive(Debug, Default, Clone)]
pub struct Trip {
    pub index: u32,
    pub id: Arc<str>,
    pub headsign: Option<Arc<str>>,
    pub short_name: Option<Arc<str>>,
}
