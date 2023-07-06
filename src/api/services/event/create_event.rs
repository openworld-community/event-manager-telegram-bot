use crate::api::services::event::types::RawEvent;
use crate::api::utils::json_response;
use crate::types::{DbPool, Event};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, Responder};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ActiveValue, DatabaseConnection};
use entity::event;
use entity::event::EventState;

#[post("")]
pub async fn create_event(
    pool: Data<DatabaseConnection>,
    event_to_create: Json<RawEvent>,
) -> actix_web::Result<impl Responder> {
    event_to_create.validation()?;

    let mut event: Event = event_to_create.into_inner().into();


    Ok(json_response(&event, StatusCode::CREATED))
}


async fn add_event(event_to_create: RawEvent, poll: &DatabaseConnection) {
    let now = Utc::now().naive_utc();

    let am = event::ActiveModel {
        id: Default::default(),
        name: ActiveValue::Set(event_to_create.name),
        link: ActiveValue::Set(event_to_create.link),
        max_adults: ActiveValue::Set(event_to_create.max_adults),
        max_children: ActiveValue::Set(event_to_create.max_children),
        max_adults_per_reservation: ActiveValue::Set(event_to_create.max_adults_per_reservation),
        max_children_per_reservation: ActiveValue::Set(event_to_create.max_children_per_reservation),
        ts: ActiveValue::Set(now),
        remind: ActiveValue::Set(event_to_create.remind.naive_utc()),
        state: ActiveValue::Set(EventState::default()),
        adult_ticket_price: ActiveValue::Set(event_to_create.adult_ticket_price),
        child_ticket_price: ActiveValue::Set(event_to_create.child_ticket_price),
        currency: ActiveValue::Set(event_to_create.currency),
    };
}
