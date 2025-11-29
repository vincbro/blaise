use std::sync::Arc;

use crate::{engine::Identifiable, gtfs::models::GtfsStop};

#[derive(Debug)]
pub enum LocationType {
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

#[derive(Debug)]
pub struct Stop {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub latitude: f64,
    pub longitude: f64,
    pub location_type: LocationType,
}

impl Identifiable for Stop {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl From<GtfsStop> for Stop {
    fn from(value: GtfsStop) -> Self {
        let location_type = if let Some(lt) = value.location_type
            && lt != 0
        {
            match lt {
                1 => LocationType::Station,
                2 => LocationType::Entrance(value.parent_station.unwrap_or("0".into()).into()),
                3 => LocationType::Node,
                4 => LocationType::Boarding,
                _ => panic!("SHOULD NEVER BE MORE THEN 4"),
            }
        } else if let Some(ps) = value.parent_station {
            let pc = value.platform_code.unwrap_or("/".into());
            LocationType::Platform {
                parent_station: ps.into(),
                platform_code: pc.into(),
            }
        } else {
            LocationType::Stop
        };

        Self {
            id: value.stop_id.into(),
            name: value.stop_name.into(),
            latitude: value.stop_lat,
            longitude: value.stop_lon,
            location_type,
        }
    }
}
