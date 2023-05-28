use crate::types::{DbPool, Event};
use std::fmt::Debug;

use crate::db::mutate_event;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, Responder};

use crate::api::shared::{into_internal_server_error_responce, QueryError};
use crate::api::utils::{json_responce, validation_error_to_http};
use chrono::{DateTime, Utc};
use tokio::task::spawn_blocking;
use validator::Validate;

fn insert_event(pool: &DbPool, event: &Event) -> Result<u64, QueryError> {
    let con = pool.get()?;
    Ok(mutate_event(&con, &event)?)
}

#[post("")]
pub async fn create_event(
    pool: Data<DbPool>,
    event_to_create: Json<RawEvent>,
) -> actix_web::Result<impl Responder> {
    event_to_create
        .validate()
        .map_err(validation_error_to_http)?;

    let mut event: Event = event_to_create.into_inner().into();
    let cloned = event.clone();
    let event_id = spawn_blocking(move || insert_event(&pool, &cloned)).await;

    event.id = event_id
        .map_err(into_internal_server_error_responce)?
        .map_err(into_internal_server_error_responce)?;

    Ok(json_responce(&event, StatusCode::CREATED))
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RawEvent {
    #[validate(length(min = 5, max = 255))]
    pub name: String,
    #[validate(url)]
    pub link: String,
    pub max_adults: i64,
    pub max_children: i64,
    pub max_adults_per_reservation: i64,
    pub max_children_per_reservation: i64,
    pub event_start_time: DateTime<Utc>,
    pub remind: DateTime<Utc>,
    pub adult_ticket_price: i64,
    pub child_ticket_price: i64,
    pub currency: String,
}

impl Into<Event> for RawEvent {
    fn into(self) -> Event {
        Event {
            id: 0,
            name: self.name,
            link: self.link,
            max_adults: self.max_adults as u64,
            max_children: self.max_children as u64,
            max_adults_per_reservation: self.max_adults_per_reservation as u64,
            max_children_per_reservation: self.max_children_per_reservation as u64,
            ts: self.event_start_time.timestamp() as u64,
            remind: self.remind.timestamp() as u64,
            adult_ticket_price: self.adult_ticket_price as u64,
            child_ticket_price: self.child_ticket_price as u64,
            currency: self.currency,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::api::services::event::create_event::RawEvent;
    use serde_json;
    use serde_json::Result;
    use validator::Validate;

    #[test]
    fn raw_event_validation() {
        let str = r#"
            {
              "name": "Test name",
              "link": "https://google.com",
              "max_adults": 2000,
              "max_children": 1000,
              "max_adults_per_reservation": 4,
              "max_children_per_reservation": 5,
              "event_start_time": "2023-05-26T17:22:00+03:00",
              "remind": "2023-05-26T16:22:00+03:00",
              "adult_ticket_price": 50,
              "child_ticket_price": 25,
              "currency": "USD"
            }
        "#;

        let event: Result<RawEvent> = serde_json::from_str(str);

        assert!(event.is_ok(), "Expected that RawEvent correctly parsed");

        let validation_result = event.unwrap().validate();

        assert!(
            validation_result.is_ok(),
            "Expected that RawEvent is valid, but got this error: {:?}",
            validation_result.err().unwrap()
        )
    }
}
