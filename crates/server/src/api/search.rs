use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::{
    dto::{AreaDto, StopDto},
    state::AppState,
};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ontrack::shared::geo::{Coordinate, Distance};

pub async fn search(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    if let Some(query) = params.get("q") {
        let count: usize = match params.get("count") {
            Some(value) => match value.parse() {
                Ok(value) => value,
                Err(_) => return Err(StatusCode::BAD_REQUEST),
            },
            None => 5,
        };
        let result: Vec<_> = state
            .repo
            .search_areas_by_name(query)
            .into_iter()
            .take(count)
            .map(|area| AreaDto::from(area, &state.repo))
            .collect();
        Ok(Json(result).into_response())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

pub async fn near(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    if let Some(query) = params.get("q") {
        let distance: Distance = match params.get("distance") {
            Some(value) => match value.parse::<f32>() {
                Ok(value) => Distance::from_meters(value),
                Err(_) => return Err(StatusCode::BAD_REQUEST),
            },
            None => Distance::from_meters(500.0),
        };
        let coordinate = Coordinate::from_str(query).map_err(|_| StatusCode::BAD_REQUEST)?;
        let mut result: Vec<_> = state
            .repo
            .areas_by_coordinate(&coordinate, distance)
            .into_iter()
            .map(|area| AreaDto::from(area, &state.repo))
            .collect();
        result.sort_by(|a, b| {
            a.coordinate
                .network_distance(&coordinate)
                .as_meters()
                .total_cmp(&b.coordinate.network_distance(&coordinate).as_meters())
        });
        Ok(Json(result).into_response())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}
