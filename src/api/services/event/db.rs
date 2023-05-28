use crate::api::shared::QueryError;
use crate::types::{Connection, Event};
use rusqlite::params;

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
