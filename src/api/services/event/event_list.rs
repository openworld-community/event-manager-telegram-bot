use crate::types::Connection;
use rusqlite::{params, Error, Row};

use crate::types::DbPool;

use actix_web::http::StatusCode;
use actix_web::web::{Data, Query};
use actix_web::{get, Responder};

use crate::api::services::event::create_event::RawEvent;
use crate::api::shared::{
    into_internal_server_error_responce, Pagination, QueryError, RawPagination, WithId,
};
use crate::api::utils::json_responce;
use crate::format::from_timestamp;
use tokio::task::spawn_blocking;

#[get("")]
pub async fn event_list(
    pool: Data<DbPool>,
    params: Query<RawPagination>,
) -> actix_web::Result<impl Responder> {
    let events = spawn_blocking(move || get_events(&pool, params.into_inner()))
        .await
        .map_err(into_internal_server_error_responce)?
        .map_err(into_internal_server_error_responce)?;

    Ok(json_responce(&events, StatusCode::OK))
}

type EventWithId = WithId<u64, RawEvent>;

fn get_events<P: Into<Pagination>>(
    pool: &DbPool,
    pagination: P,
) -> Result<Vec<EventWithId>, QueryError> {
    let con = pool.get()?;
    Ok(get_event_list(&con, pagination)?)
}

pub fn get_event_list<P: Into<Pagination>>(
    conn: &Connection,
    pagination: P,
) -> Result<Vec<EventWithId>, Error> {
    let pag = pagination.into();
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
