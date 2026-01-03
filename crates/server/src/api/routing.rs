use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ontrack::{
    repository::{Repository, StopTime},
    router::{Parent, graph::Location},
    shared::{geo::Coordinate, time::Time},
};
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

    let router = state
        .repository
        .router(from, to)
        .departure_at(Time::from_hms("16:00:00").unwrap());
    let path = router.solve().unwrap();
    println!("Length: {}", path.len());
    let mut last: Option<Parent> = path.first().cloned();
    for parent in path {
        println!("TRANSFER!");
        if let Some(last) = last
            && let Some(trip_idx) = last.trip_idx
        {
            let trip = &state.repository.trips[trip_idx as usize];
            let stop_times = &state.repository.stop_times_by_trip_id(&trip.id).unwrap();
            let mut on_trip = false;
            for i in 0..stop_times.len() {
                if stop_times[i].stop_idx == last.from_stop_idx && !on_trip {
                    on_trip = true;
                }

                let stop = &state.repository.stops[stop_times[i].stop_idx as usize];
                if on_trip {
                    println!("{}", stop.name);
                }

                if stop_times[i].stop_idx == parent.from_stop_idx && on_trip {
                    break;
                }
            }
        }

        last = Some(parent);
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
