use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use blaise::{
    repository::Repository,
    router::{Raptor, itinerary::LegType, location::Location},
    shared::{geo::Coordinate, time::Time},
};
use std::{
    collections::HashMap,
    str::{self, FromStr},
    sync::Arc,
};
use tracing::warn;

use crate::{dto::ItineraryDto, state::AppState};

pub async fn routing(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    if let Some(repository) = &*state.repository.read().await {
        let from = if let Some(from) = params.get("from") {
            location_from_str(repository, from)?
        } else {
            return Err(StatusCode::BAD_REQUEST);
        };
        let to = if let Some(to) = params.get("to") {
            location_from_str(repository, to)?
        } else {
            return Err(StatusCode::BAD_REQUEST);
        };

        let raptor =
            Raptor::new(repository, from, to).departure_at(Time::from_hms("16:00:00").unwrap());
        let itinerary = raptor.solve().unwrap();
        itinerary.legs.iter().for_each(|leg| {
            let leg_type = leg_type_str(&leg.leg_type);
            if let Location::Stop(from_stop) = &leg.from
                && let Location::Stop(to_stop) = &leg.to
            {
                let from = repository.stop_by_id(from_stop).unwrap();
                let to = repository.stop_by_id(to_stop).unwrap();
                println!(
                    "{leg_type} {} -> {} @ {} -> {}",
                    from.name,
                    to.name,
                    leg.departue_time.to_hms_string(),
                    leg.arrival_time.to_hms_string()
                );
                leg.stops.iter().for_each(|leg_stop| {
                    if let Location::Stop(stop_id) = &leg_stop.location {
                        let stop = repository.stop_by_id(stop_id).unwrap();
                        println!(
                            "| {} @ {} -> {}",
                            stop.name,
                            leg_stop.arrival_time.to_hms_string(),
                            leg_stop.departure_time.to_hms_string(),
                        );
                    }
                });
            } else if let Location::Coordinate(from_coord) = &leg.from
                && let Location::Stop(to_stop) = &leg.to
            {
                let to = repository.stop_by_id(to_stop).unwrap();
                println!(
                    "{leg_type} {} -> {} @ {} -> {}",
                    from_coord,
                    to.name,
                    leg.departue_time.to_hms_string(),
                    leg.arrival_time.to_hms_string(),
                );
            } else if let Location::Stop(from_stop) = &leg.from
                && let Location::Coordinate(to_coord) = &leg.to
            {
                let from = repository.stop_by_id(from_stop).unwrap();
                println!(
                    "{leg_type} {} -> {} @ {} -> {}",
                    from.name,
                    to_coord,
                    leg.departue_time.to_hms_string(),
                    leg.arrival_time.to_hms_string()
                );
            }
        });
        let dto =
            ItineraryDto::from(itinerary, repository).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(dto).into_response())
    } else {
        warn!("Missing repository");
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

fn location_from_str(repo: &Repository, str: &str) -> Result<Location, StatusCode> {
    if str.contains(',') {
        let coordinate = Coordinate::from_str(str).map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(coordinate.into())
    } else {
        Ok(repo.area_by_id(str).ok_or(StatusCode::BAD_REQUEST)?.into())
    }
}

fn leg_type_str(parent_type: &LegType) -> String {
    match parent_type {
        LegType::Transit => "Travel".into(),
        LegType::Transfer => "Transfer".into(),
        LegType::Walk => "Walk".into(),
    }
}
