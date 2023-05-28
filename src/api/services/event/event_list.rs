use crate::types::Connection;
use rusqlite::{params, Error, Row};

use crate::types::DbPool;

use actix_web::web::Data;
use actix_web::{get, HttpResponse, Responder};

use crate::api::services::event::create_event::RawEvent;
use crate::api::shared::WithId;
use crate::format::from_timestamp;
use tokio::task::spawn_blocking;

#[get("")]
pub async fn event_list(pool: Data<DbPool>) -> impl Responder {
    let events = spawn_blocking(move || {
        let con = pool.get().unwrap();
        get_event_list(&con).unwrap()
    })
    .await
    .expect("spawn_block error");
    HttpResponse::Ok().body(serde_json::to_string(&events).unwrap())
}

pub fn get_event_list(conn: &Connection) -> Result<Vec<TestEvent>, Error> {
    let mut stmt = conn.prepare("select * from events")?;
    let mut rows = stmt.query(params![])?;
    let mut events: Vec<TestEvent> = Vec::new();
    while let Some(row) = rows.next()? {
        events.push(map_row(&row)?);
    }

    Ok(events)
}

fn map_row(row: &Row) -> Result<TestEvent, Error> {
    Ok(TestEvent {
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

type TestEvent = WithId<u64, RawEvent>;
