use crate::api::shared::WithId;
use crate::api::utils::{validation_error_to_http, ValidationError};
use crate::types::Event;
use chrono::{DateTime, Utc};
use validator::Validate;

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

#[derive(Deserialize, Validate)]
pub struct OptionalRawEvent {
    #[validate(length(min = 5, max = 255))]
    pub name: Option<String>,
    #[validate(url)]
    pub link: Option<String>,
    pub max_adults: Option<u64>,
    pub max_children: Option<u64>,
    pub max_adults_per_reservation: Option<u64>,
    pub max_children_per_reservation: Option<u64>,
    pub event_start_time: Option<DateTime<Utc>>,
    pub remind: Option<DateTime<Utc>>,
}

pub type EventWithId = WithId<u64, RawEvent>;

impl RawEvent {
    pub fn validation(&self) -> Result<(), ValidationError> {
        self.validate().map_err(validation_error_to_http)
    }
}

impl OptionalRawEvent {
    pub fn validation(&self) -> Result<(), ValidationError> {
        self.validate().map_err(validation_error_to_http)
    }
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
    use crate::api::services::event::types::RawEvent;
    use serde_json;
    use serde_json::Result;

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

        let validation_result = event.unwrap().validation();

        assert!(
            validation_result.is_ok(),
            "Expected that RawEvent is valid, but got this error: {:?}",
            validation_result.err().unwrap()
        )
    }
}
