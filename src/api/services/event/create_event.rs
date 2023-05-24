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
    pub name: String,
    pub link: String,
    pub max_adults: i64,
    pub max_children: i64,
    pub max_adults_per_reservation: i64,
    pub max_children_per_reservation: i64,
    pub ts: DateTime<Utc>,
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
            ts: self.ts.timestamp() as u64,
            remind: self.ts.timestamp() as u64,
            adult_ticket_price: self.ts.timestamp() as u64,
            child_ticket_price: self.child_ticket_price as u64,
            currency: self.currency,
        }
    }
}
