use std::sync::Arc;

use crate::{
    repository::{Area, Repository},
    shared::geo::Coordinate,
};

#[derive(Debug, Clone)]
pub enum Location {
    Area(Arc<str>),
    Stop(Arc<str>),
    Coordinate(Coordinate),
}

impl From<&Area> for Location {
    fn from(value: &Area) -> Self {
        Self::Area(value.id.clone())
    }
}

impl From<Area> for Location {
    fn from(value: Area) -> Self {
        Self::Area(value.id)
    }
}

impl From<Coordinate> for Location {
    fn from(value: Coordinate) -> Self {
        Self::Coordinate(value)
    }
}
