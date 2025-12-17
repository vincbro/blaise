use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ontrack::engine::{
    Engine,
    geo::Coordinate,
    routing::graph::{Location, SearchState, Transition},
};
use std::{collections::HashMap, sync::Arc};

use crate::state::AppState;

pub async fn routing(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    let from: Location = if let Some(from) = params.get("from") {
        waypoint_from_str(&state.engine, from)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };
    dbg!(&from);
    let to: Location = if let Some(to) = params.get("to") {
        waypoint_from_str(&state.engine, to)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };
    dbg!(&to);

    let router = state
        .engine
        .router(from, to)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let itinerary = router.run().map_err(|_| StatusCode::BAD_REQUEST)?;
    for leg in itinerary.legs.into_iter() {
        println!(
            "{} from {} to {} with {} steps",
            leg.mode,
            get_name(&leg.from, &state.engine),
            get_name(&leg.to, &state.engine),
            leg.instructions.len()
        );
    }
    // dbg!(itinerary);
    Ok("SUP".into_response())
}

fn waypoint_from_str(engine: &Engine, str: &str) -> Result<Location, StatusCode> {
    if str.contains(',') {
        let split: Vec<_> = str.split(',').collect();
        let latitude: f64 = split
            .first()
            .ok_or(StatusCode::BAD_REQUEST)?
            .parse()
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let longitude: f64 = split
            .last()
            .ok_or(StatusCode::BAD_REQUEST)?
            .parse()
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(Coordinate {
            latitude,
            longitude,
        }
        .into())
    } else {
        Ok(engine
            .area_by_id(str)
            .ok_or(StatusCode::BAD_REQUEST)?
            .into())
    }
}

// TEMP
fn get_name(location: &Location, engine: &Engine) -> String {
    match location {
        Location::Area(id) => engine
            .area_by_id(id)
            .map(|value| value.name.to_string())
            .unwrap_or(id.to_string()),
        Location::Stop(id) => engine
            .stop_by_id(id)
            .map(|value| value.name.to_string())
            .unwrap_or(id.to_string()),

        Location::Coordinate(coordinate) => coordinate.to_string(),
    }
}

// TEMP
fn get_mode(state: &SearchState) -> String {
    match state.transition {
        Transition::Transit { .. } => "Traveled".to_string(),
        Transition::Walk => "Walked".to_string(),
        Transition::Transfer { .. } => "Transfered".to_string(),
        Transition::Genesis => "Genesis".to_string(),
    }
}
