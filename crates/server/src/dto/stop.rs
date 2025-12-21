use ontrack::{repository::Stop, shared::geo::Coordinate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopDto {
    pub id: String,
    pub name: String,
    pub coordinate: Coordinate,
}

impl StopDto {
    pub fn from(stop: &Stop) -> Self {
        let id = stop.id.to_string();
        let name = stop.name.to_string();
        let coordinate = stop.coordinate;
        Self {
            id,
            name,
            coordinate,
        }
    }
}
