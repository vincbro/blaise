use std::sync::Arc;

use crate::gtfs::models::GtfsTrip;

#[derive(Debug, Default, Clone)]
pub struct Trip {
    pub index: usize,
    pub id: Arc<str>,
    pub headsign: Option<Arc<str>>,
    pub short_name: Option<Arc<str>>,
}

impl From<GtfsTrip> for Trip {
    fn from(value: GtfsTrip) -> Self {
        Self {
            index: usize::MAX,
            id: value.trip_id.into(),
            headsign: value.trip_headsign.map(|val| val.into()),
            short_name: value.trip_short_name.map(|val| val.into()),
        }
    }
}
