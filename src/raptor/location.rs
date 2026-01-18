use crate::{repository::Area, shared::geo::Coordinate};
use std::sync::Arc;

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

#[derive(Debug, Clone, Copy)]
pub enum Point {
    Coordinate(Coordinate),
    Stop(u32),
}

impl From<u32> for Point {
    fn from(value: u32) -> Self {
        Self::Stop(value)
    }
}

impl From<Coordinate> for Point {
    fn from(value: Coordinate) -> Self {
        Self::Coordinate(value)
    }
}
