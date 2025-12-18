use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ontrack::engine::{Engine, geo::Coordinate, routing::graph::Location};
use std::{collections::HashMap, sync::Arc};

use crate::{dto::ItineraryDto, state::AppState};

pub async fn routing(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    let from: Location = if let Some(from) = params.get("from") {
        location_from_str(&state.engine, from)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };
    dbg!(&from);
    let to: Location = if let Some(to) = params.get("to") {
        location_from_str(&state.engine, to)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };
    dbg!(&to);

    let router = state
        .engine
        .router(from, to)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let itinerary = router.run().map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(ItineraryDto::from(itinerary, &state.engine)).into_response())
}

fn location_from_str(engine: &Engine, str: &str) -> Result<Location, StatusCode> {
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
