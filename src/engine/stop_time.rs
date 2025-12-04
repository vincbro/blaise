use std::sync::Arc;

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
    pub trip_id: Arc<str>,
    pub arrival_time: Arc<str>,
    pub departure_time: Arc<str>,
    pub stop_id: Arc<str>,
    pub stop_sequence: i64,
    pub stop_headsign: Option<Arc<str>>,
    pub shape_dist_traveled: Option<f64>,
    pub pickup_type: StopAccessType,
    pub drop_off_type: StopAccessType,
    pub timepoint: Timepoint,
}
