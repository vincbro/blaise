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

    let coord = match from {
        Location::Area(id) => state
            .repository
            .coordinate_by_area_id(&id)
            .ok_or(StatusCode::BAD_REQUEST),
        Location::Stop(id) => state
            .repository
            .stop_by_id(&id)
            .map(|stop| stop.coordinate)
            .ok_or(StatusCode::BAD_REQUEST),
        Location::Coordinate(coordinate) => Ok(coordinate),
    }?;

    let stops = state.repository.stops_by_coordinate(&coord, 500.0.into());

    let trips: Vec<_> = stops
        .into_iter()
        .filter_map(|stop| state.repository.trips_by_stop_id(&stop.id))
        .flatten()
        .collect();
    if !trips.is_empty() {
        let trip = trips[0];
        let stop_times = state.repository.stop_times_by_trip_id(&trip.id).unwrap();

        for stop_time in stop_times {
            let stop = state.repository.stop_by_id(&stop_time.stop_id).unwrap();
            println!(
                "[{}] start: {} | idx: {} | stop: {} | valid: {}",
                stop_time.index,
                stop_time.start_idx,
                stop_time.internal_idx,
                stop.name,
                stop_time.index == stop_time.start_idx + stop_time.internal_idx
            )
        }
    } else {
        println!("EMPTY");
    }

    return Ok("HELLO".into_response());
    // let router = state.repository.router(from, to);
    // router.solve().unwrap();
    // // Ok(Json(ItineraryDto::from(itinerary, &state.repo)).into_response())
    // Ok("HELLO".into_response())
}

fn location_from_str(repo: &Repository, str: &str) -> Result<Location, StatusCode> {
    if str.contains(',') {
        let coordinate = Coordinate::from_str(str).map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(coordinate.into())
    } else {
        Ok(repo.area_by_id(str).ok_or(StatusCode::BAD_REQUEST)?.into())
    }
}
