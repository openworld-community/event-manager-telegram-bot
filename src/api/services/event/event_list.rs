use crate::types::DbPool;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Query};
use actix_web::{get, Responder};
use rusqlite::{params, Error, Row};
use crate::api::services::event::types::{EventWithId, RawEvent};
use crate::api::shared::{
    into_internal_server_error_responce, Pagination, QueryError, RawPagination,
};
use crate::api::utils::json_responce;
use crate::format::from_timestamp;
use tokio::task::spawn_blocking;

#[get("")]
pub async fn event_list(
    pool: Data<DbPool>,
    params: Query<RawPagination>,
) -> actix_web::Result<impl Responder> {
    let events = spawn_blocking(move || get_event_list(&pool, &params.into_inner().into()))
        .await
        .map_err(into_internal_server_error_responce)?
        .map_err(into_internal_server_error_responce)?;

    Ok(json_responce(&events, StatusCode::OK))
}

pub fn get_event_list(pool: &DbPool, pag: &Pagination) -> Result<Vec<EventWithId>, QueryError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("select * from events limit ? offset ?")?;
    let mut rows = stmt.query(params![pag.limit(), pag.offset()])?;
    let mut events: Vec<EventWithId> = Vec::new();
    while let Some(row) = rows.next()? {
        events.push(map_row(&row)?);
    }

    Ok(events)
}

fn map_row(row: &Row) -> Result<EventWithId, Error> {
    Ok(EventWithId {
        id: row.get("id")?,
        entity: RawEvent {
            name: row.get("name")?,
            link: row.get("link")?,
            max_adults: row.get("max_adults")?,
            max_children: row.get("max_children")?,
            max_adults_per_reservation: row.get("max_adults_per_reservation")?,
            max_children_per_reservation: row.get("max_children_per_reservation")?,
            event_start_time: from_timestamp(row.get("ts")?),
            remind: from_timestamp(row.get("remind")?),
            adult_ticket_price: row.get("adult_ticket_price")?,
            child_ticket_price: row.get("child_ticket_price")?,
            currency: row.get("currency")?,
        },
    })
}
