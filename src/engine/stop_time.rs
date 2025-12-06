use std::sync::Arc;

use crate::gtfs::models::GtfsStopTime;

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
    pub index: i64,
    pub arrival_time: Arc<str>,
    pub departure_time: Arc<str>,
    pub headsign: Option<Arc<str>>,
    pub dist_traveled: Option<f64>,
    pub pickup_type: StopAccessType,
    pub drop_off_type: StopAccessType,
    pub timepoint: Timepoint,
}

impl From<GtfsStopTime> for StopTime {
    fn from(value: GtfsStopTime) -> Self {
        Self {
            index: value.stop_sequence,
            arrival_time: value.arrival_time.into(),
            departure_time: value.departure_time.into(),
            headsign: value.stop_headsign.map(|val| val.into()),
            dist_traveled: value.shape_dist_traveled.into(),
            pickup_type: StopAccessType::Regularly,
            drop_off_type: StopAccessType::Regularly,
            timepoint: Timepoint::Exact,
        }
    }
}
