use std::sync::Arc;

use crate::{engine::geo::Distance, gtfs::models::GtfsStopTime};

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
    pub trip_idx: usize,
    pub trip_id: Arc<str>,
    pub stop_idx: usize,
    pub stop_id: Arc<str>,
    pub sequence: usize,
    pub arrival_time: Arc<str>,
    pub departure_time: Arc<str>,
    pub headsign: Option<Arc<str>>,
    pub dist_traveled: Option<Distance>,
    pub pickup_type: StopAccessType,
    pub drop_off_type: StopAccessType,
    pub timepoint: Timepoint,
}

impl From<GtfsStopTime> for StopTime {
    fn from(value: GtfsStopTime) -> Self {
        Self {
            trip_id: Default::default(),
            trip_idx: usize::MAX,
            stop_id: Default::default(),
            stop_idx: usize::MAX,
            sequence: value.stop_sequence as usize,
            arrival_time: value.arrival_time.into(),
            departure_time: value.departure_time.into(),
            headsign: value.stop_headsign.map(|val| val.into()),
            dist_traveled: value.shape_dist_traveled.map(Distance::kilometers),
            pickup_type: StopAccessType::Regularly,
            drop_off_type: StopAccessType::Regularly,
            timepoint: Timepoint::Exact,
        }
    }
}
