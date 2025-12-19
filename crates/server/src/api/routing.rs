use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ontrack::{
    repository::Repository,
    router::{Router, graph::Location},
    shared::{geo::Coordinate, time::Time},
};
use std::{
    collections::HashMap,
    str::{self, FromStr},
    sync::Arc,
};

use crate::{dto::ItineraryDto, state::AppState};

pub async fn routing(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    let from = if let Some(from) = params.get("from") {
        location_from_str(&state.repo, from)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };
    let to = if let Some(to) = params.get("to") {
        location_from_str(&state.repo, to)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let router = Router::new(
        state.repo.clone(),
        from,
        to,
        Time::from_hms("16:00:00").unwrap(),
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;
    let itinerary = router.run().map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(ItineraryDto::from(itinerary, &state.repo)).into_response())
}

fn location_from_str(repo: &Repository, str: &str) -> Result<Location, StatusCode> {
    if str.contains(',') {
        let coordinate = Coordinate::from_str(str).map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(coordinate.into())
    } else {
        Ok(repo.area_by_id(str).ok_or(StatusCode::BAD_REQUEST)?.into())
    }
}
