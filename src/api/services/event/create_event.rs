use crate::types::{DbPool, Event};

use crate::db::add_event;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use tokio::task::spawn_blocking;
use validator::Validate;

#[post("")]
pub async fn create_event(pool: Data<DbPool>, event_to_create: Json<RawEvent>) -> impl Responder {
    event_to_create.validate().unwrap();
    let mut event = event_to_create.into_inner().to_event();
    let cloned = event.clone();
    let event_id = spawn_blocking(move || {
        let con = pool.get().unwrap();
        add_event(&con, cloned).unwrap()
    })
    .await
    .unwrap();
    event.id = event_id;
    HttpResponse::Created().body(serde_json::to_string(&event).unwrap())
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

impl RawEvent {
    fn to_event(self) -> Event {
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
