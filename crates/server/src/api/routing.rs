use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ontrack::{repository::Repository, router::graph::Location, shared::geo::Coordinate};
use std::{
    collections::HashMap,
    str::{self, FromStr},
    sync::Arc,
};

use crate::state::AppState;

pub async fn routing(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    let from = if let Some(from) = params.get("from") {
        location_from_str(&state.repository, from)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };
    let to = if let Some(to) = params.get("to") {
        location_from_str(&state.repository, to)?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let coord = match &from {
        Location::Area(id) => state
            .repository
            .coordinate_by_area_id(id)
            .ok_or(StatusCode::BAD_REQUEST),
        Location::Stop(id) => state
            .repository
            .stop_by_id(id)
            .map(|stop| stop.coordinate)
            .ok_or(StatusCode::BAD_REQUEST),
        Location::Coordinate(coordinate) => Ok(*coordinate),
    }?;

    state
        .repository
        .stops_by_coordinate(&coord, 500.0.into())
        .into_iter()
        .for_each(|stop| {
            println!("{}", stop.name);
        });

    let router = state.repository.router(from, to);
    let path = router.solve().unwrap();
    println!("Length: {}", path.len());
    for parent in path {
        if parent.from_stop_idx == u32::MAX {
            println!("START");
        } else {
            let stop = &state.repository.stops[parent.from_stop_idx as usize];
            println!("{}", stop.name);
        }
    }
    // Ok(Json(ItineraryDto::from(itinerary, &state.repo)).into_response())
    Ok("HELLO".into_response())
}

fn location_from_str(repo: &Repository, str: &str) -> Result<Location, StatusCode> {
    if str.contains(',') {
        let coordinate = Coordinate::from_str(str).map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(coordinate.into())
    } else {
        Ok(repo.area_by_id(str).ok_or(StatusCode::BAD_REQUEST)?.into())
    }
}
