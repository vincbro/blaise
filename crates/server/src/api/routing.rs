use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ontrack::engine::{
    Engine,
    geo::Coordinate,
    routing::{
        Waypoint,
        graph::{SearchState, Transition},
    },
};
use std::{collections::HashMap, sync::Arc};

use crate::state::AppState;

pub async fn routing(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    let from: Waypoint = if let Some(from) = params.get("from") {
        waypoint_from_str(&state.engine, from)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };
    dbg!(&from);
    let to: Waypoint = if let Some(to) = params.get("to") {
        waypoint_from_str(&state.engine, to)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };
    dbg!(&to);

    let mut router = state
        .engine
        .router(from, to)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let route = router.run().map_err(|_| StatusCode::BAD_REQUEST)?;

    let steps = route.len();
    let mut respone: Vec<String> = Vec::with_capacity(steps);
    route
        .into_iter()
        .take(steps - 1)
        .skip(1)
        .for_each(|search_state| {
            respone.push(format!(
                "{} from {} to {} we are at {} seconds",
                get_mode(&search_state),
                get_name(&search_state.parent.clone().unwrap(), &state.engine),
                get_name(&search_state, &state.engine),
                search_state.g_time
            ));
        });
    Ok(Json(respone).into_response())
}

fn waypoint_from_str(engine: &Engine, str: &str) -> Result<Waypoint, StatusCode> {
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
fn get_name(state: &SearchState, engine: &Engine) -> String {
    match state.stop_idx {
        Some(stop_idx) => engine.stops[stop_idx].name.to_string(),
        None => format!(
            "{}, {}",
            state.coordinate.latitude, state.coordinate.longitude,
        ),
    }
}

// TEMP
fn get_mode(state: &SearchState) -> String {
    match state.transition {
        Transition::Travel { .. } => "Traveled".to_string(),
        Transition::Walk => "Walked".to_string(),
        Transition::Transfer { .. } => "Transfered".to_string(),
        Transition::Genesis => "Genesis".to_string(),
    }
}
