use std::{collections::HashMap, sync::Arc};

use crate::state::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ontrack::engine::geo::Coordinate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SearchResult {
    pub name: String,
    pub coordinate: Coordinate,
}

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
            .engine
            .search_areas_by_name(query)
            .into_iter()
            .take(count)
            .map(|area| {
                let name = area.name.to_string();
                let coordinate: Coordinate = state
                    .engine
                    .stops_by_area_id(&area.id)
                    .unwrap()
                    .into_iter()
                    .map(|stop| stop.coordinate)
                    .sum();
                SearchResult { name, coordinate }
            })
            .collect();
        Ok(Json(result).into_response())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}
