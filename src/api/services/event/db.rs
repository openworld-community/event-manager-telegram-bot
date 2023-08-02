use crate::api::services::event::types::{EventWithId, RawEvent};
use crate::api::shared::{Pagination, QueryError};
use crate::format::from_timestamp;
use crate::types::{Connection, DbPool, Event};
use rusqlite::{params, Error, Row};

pub async fn select_event(conn: &Connection, id: i64) -> Result<Event, QueryError> {
    let mut stmt = conn
        .query_one("select * from events where id=$1", &[id])
        .await?
        .map_err(|e| QueryError::NotFound(e.to_string()))?;
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

pub async fn get_event_list(
    pool: &DbPool,
    pag: &Pagination,
) -> Result<Vec<EventWithId>, QueryError> {
    let conn = pool.get().await.unwrap();
    let mut stmt = conn
        .prepare("select * from events limit $1 offset $2")
        .await?;
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
