use std::sync::Arc;

use crate::{engine::Identifiable, gtfs::models::GtfsArea};

#[derive(Debug, Default, Clone)]
pub struct Area {
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

impl From<GtfsArea> for Area {
    fn from(value: GtfsArea) -> Self {
        Self {
            id: value.area_id.into(),
            name: value.area_name.clone().into(),
            normalized_name: value.area_name.to_lowercase().into(),
        }
    }
}
