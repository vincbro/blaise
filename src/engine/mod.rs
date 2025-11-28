use std::{collections::HashMap, sync::Arc};

mod area;
pub mod fuzzy;
mod stop;
pub use area::*;
pub use stop::*;

use crate::gtfs::Gtfs;

#[derive(Clone, Default)]
pub struct Engine {
    stops: Arc<[Stop]>,
    areas: Arc<[Area]>,
    stop_lookup: Arc<HashMap<Arc<str>, usize>>,
    area_lookup: Arc<HashMap<Arc<str>, usize>>,
    area_to_stops: Arc<HashMap<Arc<str>, Vec<Arc<str>>>>,
    stop_to_area: Arc<HashMap<Arc<str>, Arc<str>>>,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_gtfs(mut self, gtfs: Gtfs) -> Self {
        // Build stop db
        let mut stop_lookup: HashMap<Arc<str>, usize> = HashMap::new();
        self.stops = gtfs
            .stops
            .into_iter()
            .enumerate()
            .map(|(i, stop)| {
                let value: Stop = stop.into();
                stop_lookup.insert(value.id.clone(), i);
                value
            })
            .collect::<Arc<[Stop]>>();
        self.stop_lookup = stop_lookup.into();

        // Build area db
        let mut area_lookup: HashMap<Arc<str>, usize> = HashMap::new();
        self.areas = gtfs
            .areas
            .into_iter()
            .enumerate()
            .map(|(i, area)| {
                let value: Area = area.into();
                area_lookup.insert(value.id.clone(), i);
                value
            })
            .collect::<Arc<[Area]>>();
        self.area_lookup = area_lookup.into();

        let mut area_to_stops: HashMap<Arc<str>, Vec<Arc<str>>> = HashMap::new();
        let mut stop_to_area: HashMap<Arc<str>, Arc<str>> = HashMap::new();
        gtfs.stop_areas.into_iter().for_each(|value| {
            //TEMP
            let stop_index = self.stop_lookup.get(value.stop_id.as_str()).unwrap();
            let stop_id = self.stops[*stop_index].id.clone();
            //TEMP
            let area_index = self.area_lookup.get(value.area_id.as_str()).unwrap();
            let area_id = self.areas[*area_index].id.clone();

            stop_to_area.insert(stop_id.clone(), area_id.clone());
            if let Some(stops) = area_to_stops.get_mut(&area_id) {
                stops.push(stop_id);
            } else {
                area_to_stops.insert(area_id, vec![stop_id]);
            }
        });
        self.stop_to_area = stop_to_area.into();
        self.area_to_stops = area_to_stops.into();
        println!(
            "stops: {} | stop_to_area: {}",
            self.stops.len(),
            self.stop_to_area.len()
        );
        self
    }

    pub fn get_area(&self, id: &str) -> Option<&Area> {
        let area_index = self.area_lookup.get(id)?;
        Some(&self.areas[*area_index])
    }

    pub fn get_stop(&self, id: &str) -> Option<&Stop> {
        let stop_index = self.stop_lookup.get(id)?;
        Some(&self.stops[*stop_index])
    }

    pub fn get_stops_in_area(&self, id: &str) -> Option<Vec<&Stop>> {
        let stops = self.area_to_stops.get(id)?;
        Some(
            stops
                .iter()
                .filter_map(|stop_id| self.get_stop(stop_id))
                .collect(),
        )
    }

    pub fn get_area_from_stop(&self, id: &str) -> Option<&Area> {
        let area_id = self.stop_to_area.get(id)?;
        self.get_area(area_id)
    }
}
