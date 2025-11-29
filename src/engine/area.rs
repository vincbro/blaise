use std::sync::Arc;

use crate::{engine::Identifiable, gtfs::models::GtfsArea};

pub struct Area {
    pub id: Arc<str>,
    pub name: Arc<str>,
}

impl Identifiable for Area {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl From<GtfsArea> for Area {
    fn from(value: GtfsArea) -> Self {
        Self {
            id: value.area_id.into(),
            name: value.area_name.into(),
        }
    }
}
