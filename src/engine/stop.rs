use std::sync::Arc;

use crate::{
    engine::{CELL_SIZE_METER, Identifiable},
    gtfs::models::GtfsStop,
};

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

#[derive(Debug, Default, Clone, Copy)]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

impl Coordinate {
    pub fn distance_km(&self, coord: &Self) -> f64 {
        const R: f64 = 6371.0;
        let dist_lat = f64::to_radians(coord.latitude - self.latitude);
        let dist_lon = f64::to_radians(coord.longitude - self.longitude);
        let a = f64::powi(f64::sin(dist_lat / 2.0), 2)
            + f64::cos(f64::to_radians(self.latitude))
                * f64::cos(f64::to_radians(coord.latitude))
                * f64::sin(dist_lon / 2.0)
                * f64::sin(dist_lon / 2.0);
        let c = 2.0 * f64::atan2(f64::sqrt(a), f64::sqrt(1.0 - a));
        R * c
    }

    pub fn distance_m(&self, coord: &Self) -> f64 {
        self.distance_km(coord) * 1000.0
    }

    pub fn to_grid(&self) -> (i32, i32) {
        let x = (self.longitude * 111_320.0 / CELL_SIZE_METER) as i32;
        let y = (self.latitude * 110_540.0 / CELL_SIZE_METER) as i32;
        (x, y)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Stop {
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
            name: value.stop_name.clone().into(),
            normalized_name: value.stop_name.to_lowercase().into(),
            coordinate: Coordinate {
                latitude: value.stop_lat,
                longitude: value.stop_lon,
            },
            location_type,
        }
    }
}
