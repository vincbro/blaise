use ontrack::{
    repository::{Area, Repository},
    shared::geo::Coordinate,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaDto {
    pub id: String,
    pub name: String,
    pub coordinate: Coordinate,
}

impl AreaDto {
    pub fn from(area: &Area, repo: &Repository) -> Self {
        let id = area.id.to_string();
        let name = area.name.to_string();
        let coordinate: Coordinate = repo
            .stops_by_area_id(&area.id)
            .unwrap()
            .into_iter()
            .map(|stop| stop.coordinate)
            .sum();
        Self {
            id,
            name,
            coordinate,
        }
    }
}
