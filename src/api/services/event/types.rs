use crate::api::shared::WithId;
use crate::api::shared::ValidationError;
use crate::types::Event;
use chrono::{DateTime, Utc};
use std::borrow::Cow;
use std::collections::HashMap;
use validator::{Validate, ValidationErrors};

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RawEvent {
    #[validate(length(min = 5, max = 255))]
    pub name: String,
    #[validate(url)]
    pub link: String,
    pub max_adults: i32,
    pub max_children: i32,
    pub max_adults_per_reservation: i32,
    pub max_children_per_reservation: i32,
    pub event_start_time: DateTime<Utc>,
    pub remind: DateTime<Utc>,
    pub adult_ticket_price: i32,
    pub child_ticket_price: i32,
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
    pub adult_ticket_price: Option<u64>,
    pub child_ticket_price: Option<u64>,
    pub currency: Option<String>,
}

pub type EventWithId = WithId<u64, RawEvent>;

impl RawEvent {
    pub fn validation(&self) -> Result<(), ValidationError> {
        let mut errors = match self.validate() {
            Ok(_) => ValidationErrors::new(),
            Err(err) => err,
        };

        if self.max_adults == 0 && self.adult_ticket_price != 0 {
            errors.add(
                "adult_ticket_price",
                validator::ValidationError {
                    code: Cow::from("adult_ticket_price"),
                    message: Some(Cow::from(
                        "adult_ticket_price should be 0 when max_adults is 0",
                    )),
                    params: HashMap::new(),
                },
            )
        }

        if self.max_children == 0 && self.child_ticket_price != 0 {
            errors.add(
                "child_ticket_price",
                validator::ValidationError {
                    code: Cow::from("child_ticket_price"),
                    message: Some(Cow::from(
                        "child_ticket_price should be 0 when max_children is 0",
                    )),
                    params: HashMap::new(),
                },
            )
        }

        if self.max_adults_per_reservation > self.max_adults {
            errors.add(
                "max_adults_per_reservation",
                validator::ValidationError {
                    code: Cow::from("max_adults_per_reservation"),
                    message: Some(Cow::from(
                        "max_adults_per_reservation count mast be less then max_adults",
                    )),
                    params: HashMap::new(),
                },
            )
        }

        if self.max_children_per_reservation > self.max_children {
            errors.add(
                "max_children_per_reservation",
                validator::ValidationError {
                    code: Cow::from("max_children_per_reservation"),
                    message: Some(Cow::from(
                        "max_children_per_reservation count mast be less then max_children",
                    )),
                    params: HashMap::new(),
                },
            )
        }

        match errors.is_empty() {
            true => Ok(()),
            false => Err(errors.into()),
        }
    }
}

impl OptionalRawEvent {
    pub fn validation(&self, current_event: &Event) -> Result<(), ValidationError> {
        let mut errors = match self.validate() {
            Ok(_) => ValidationErrors::new(),
            Err(err) => err,
        };

        if self.get_max_adults(&current_event) == 0
            && self.get_adult_ticket_price(&current_event) != 0
        {
            errors.add(
                "adult_ticket_price",
                validator::ValidationError {
                    code: Cow::from("adult_ticket_price"),
                    message: Some(Cow::from(
                        "adult_ticket_price should be 0 when max_adults is 0",
                    )),
                    params: HashMap::new(),
                },
            )
        }

        if self.get_max_children(&current_event) == 0
            && self.get_child_ticket_price(&current_event) != 0
        {
            errors.add(
                "child_ticket_price",
                validator::ValidationError {
                    code: Cow::from("child_ticket_price"),
                    message: Some(Cow::from(
                        "child_ticket_price should be 0 when max_children is 0",
                    )),
                    params: HashMap::new(),
                },
            )
        }

        if self.get_max_adults_per_reservation(&current_event) > self.get_max_adults(&current_event)
        {
            errors.add(
                "max_adults_per_reservation",
                validator::ValidationError {
                    code: Cow::from("max_adults_per_reservation"),
                    message: Some(Cow::from(
                        "max_adults_per_reservation count mast be less then max_adults",
                    )),
                    params: HashMap::new(),
                },
            )
        }

        if self.get_max_children_per_reservation(&current_event)
            > self.get_max_children(&current_event)
        {
            errors.add(
                "max_children_per_reservation",
                validator::ValidationError {
                    code: Cow::from("max_children_per_reservation"),
                    message: Some(Cow::from(
                        "max_children_per_reservation count mast be less then max_children",
                    )),
                    params: HashMap::new(),
                },
            )
        }

        match errors.is_empty() {
            true => Ok(()),
            false => Err(errors.into()),
        }
    }

    fn get_max_adults(&self, current_event: &Event) -> u64 {
        self.max_adults.unwrap_or(current_event.max_adults)
    }

    fn get_max_children(&self, current_event: &Event) -> u64 {
        self.max_children.unwrap_or(current_event.max_children)
    }

    fn get_adult_ticket_price(&self, current_event: &Event) -> u64 {
        self.adult_ticket_price
            .unwrap_or(current_event.adult_ticket_price)
    }

    fn get_child_ticket_price(&self, current_event: &Event) -> u64 {
        self.child_ticket_price
            .unwrap_or(current_event.child_ticket_price)
    }

    fn get_max_adults_per_reservation(&self, current_event: &Event) -> u64 {
        self.max_adults_per_reservation
            .unwrap_or(current_event.max_adults_per_reservation)
    }

    fn get_max_children_per_reservation(&self, current_event: &Event) -> u64 {
        self.max_children_per_reservation
            .unwrap_or(current_event.max_children_per_reservation)
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
