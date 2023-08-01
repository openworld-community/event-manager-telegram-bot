use crate::api::services::event::types::{EventWithId, RawEvent};
use crate::api::shared::{Pagination, QueryError};
use crate::format::from_timestamp;
use crate::types::{Connection, DbPool, Event};
use rusqlite::{params, Error, Row};

pub fn select_event(conn: &Connection, id: i64) -> Result<Event, QueryError> {
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

pub fn get_event_list(pool: &DbPool, pag: &Pagination) -> Result<Vec<EventWithId>, QueryError> {
    let conn = poolget().await;
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
