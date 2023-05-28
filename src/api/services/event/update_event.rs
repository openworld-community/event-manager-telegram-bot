use crate::api::services::event::types::OptionalRawEvent;
use crate::api::shared::{into_internal_server_error_responce, QueryError};
use crate::api::utils::json_responce;
use crate::db::mutate_event;
use crate::types::{DbPool, Event};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json, Path};
use actix_web::{post, Responder};
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use tokio::task::spawn_blocking;

#[post("/{id}")]
pub async fn update_event(
    id: Path<i64>,
    event_to_update: Json<OptionalRawEvent>,
    pool: Data<DbPool>,
) -> actix_web::Result<impl Responder> {
    event_to_update.validation()?;

    let new_event = spawn_blocking(move || {
        perform_update_event(
            &pool.into_inner(),
            id.into_inner(),
            event_to_update.into_inner(),
        )
    })
    .await
    .map_err(into_internal_server_error_responce)?
    .map_err(into_internal_server_error_responce)?;

    Ok(json_responce(&new_event, StatusCode::OK))
}

pub fn select_event(
    conn: &PooledConnection<SqliteConnectionManager>,
    id: i64,
) -> Result<Event, QueryError> {
    let mut stmt = conn.prepare("select * from events where id=?1")?;
    let mut result = stmt.query(params![id])?;
    let some_row = result.next()?;

    let row = some_row.unwrap();

    Ok(Event {
        id: row.get("id")?,
        name: row.get("name")?,
        link: row.get("link")?,
        max_adults: row.get("max_adults")?,
        max_children: row.get("max_children")?,
        max_adults_per_reservation: row.get("max_adults_per_reservation")?,
        max_children_per_reservation: row.get("max_children_per_reservation")?,
        ts: row.get("ts")?,
        remind: row.get("remind")?,
        adult_ticket_price: row.get("adult_ticket_price")?,
        child_ticket_price: row.get("child_ticket_price")?,
        currency: row.get("currency")?,
    })
}

pub fn perform_update_event(
    pool: &DbPool,
    id: i64,
    event_to_update: OptionalRawEvent,
) -> Result<Event, QueryError> {
    let conn = pool.get()?;

    let current_event = select_event(&conn, id)?;

    let new_event = Event {
        id: id as u64,
        name: event_to_update.name.unwrap_or(current_event.name),
        link: event_to_update.link.unwrap_or(current_event.link),
        max_adults: event_to_update
            .max_adults
            .unwrap_or(current_event.max_adults),
        max_children: event_to_update
            .max_children
            .unwrap_or(current_event.max_children),
        max_adults_per_reservation: event_to_update
            .max_adults_per_reservation
            .unwrap_or(current_event.max_adults_per_reservation),
        max_children_per_reservation: event_to_update
            .max_children_per_reservation
            .unwrap_or(current_event.max_children_per_reservation),
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
        currency: current_event.currency,
    };

    mutate_event(&conn, &new_event)?;

    Ok(new_event)
}
