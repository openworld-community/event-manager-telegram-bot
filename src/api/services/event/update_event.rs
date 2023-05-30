use std::sync::Arc;
use crate::api::services::event::db;
use crate::api::services::event::types::OptionalRawEvent;
use crate::api::shared::{into_internal_server_error_responce, QueryError};
use crate::api::utils::json_responce;
use crate::db::mutate_event;
use crate::types::{DbPool, Event};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json, Path};
use actix_web::{post, Responder};
use tokio::task::spawn_blocking;

#[post("/{id}")]
pub async fn update_event(
    id: Path<i64>,
    event_to_update: Json<OptionalRawEvent>,
    pool: Data<DbPool>,
) -> actix_web::Result<impl Responder> {
    let id = id.into_inner();

    let pool_for_current_event= pool.clone();
    let current_event = spawn_blocking(move || {
        let conn = pool_for_current_event.get()?;
        db::select_event(&conn, id)
    })
        .await
        .map_err(into_internal_server_error_responce)?
        .map_err(into_internal_server_error_responce)?;

    event_to_update.validation(&current_event)?;

    let new_event = spawn_blocking(move || {
        perform_update_event(
            &pool.into_inner(),
            id,
            event_to_update.into_inner(),
            &current_event
        )
    })
    .await
    .map_err(into_internal_server_error_responce)?
    .map_err(into_internal_server_error_responce)?;

    Ok(json_responce(&new_event, StatusCode::OK))
}

pub fn perform_update_event(
    pool: &DbPool,
    id: i64,
    event_to_update: OptionalRawEvent,
    current_event: &Event,
) -> Result<Event, QueryError> {
    let conn = pool.get()?;

    let new_event = Event {
        id: id as u64,
        name: event_to_update.name.unwrap_or(current_event.name.clone()),
        link: event_to_update.link.unwrap_or(current_event.link.clone()),
        max_adults: event_to_update
            .max_adults
            .unwrap_or(current_event.max_adults as i64) as u64,
        max_children: event_to_update
            .max_children
            .unwrap_or(current_event.max_children as i64) as u64,
        max_adults_per_reservation: event_to_update
            .max_adults_per_reservation
            .unwrap_or(current_event.max_adults_per_reservation as i64) as u64,
        max_children_per_reservation: event_to_update
            .max_children_per_reservation
            .unwrap_or(current_event.max_children_per_reservation as i64) as u64,
        ts: event_to_update
            .event_start_time
            .map(|val| val.timestamp() as u64)
            .unwrap_or(current_event.ts),
        remind: event_to_update
            .remind
            .map(|val| val.timestamp() as u64)
            .unwrap_or(current_event.remind),
        adult_ticket_price: current_event.adult_ticket_price,
        child_ticket_price: current_event.child_ticket_price,
        currency: current_event.currency.clone(),
    };

    mutate_event(&conn, &new_event)?;

    Ok(new_event)
}
