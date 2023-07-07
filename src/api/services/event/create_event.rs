use crate::api::services::event::types::RawEvent;
use crate::api::shared::AppError;
use crate::api::utils::json_response;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, Responder};
use chrono::Utc;
use entity::event;
use entity::event::EventState;
use sea_orm::{ActiveValue, DatabaseConnection, DbErr, EntityTrait};


#[post("")]
pub async fn create_event(
    pool: Data<DatabaseConnection>,
    event_to_create: Json<RawEvent>,
) -> Result<impl Responder, AppError> {
    event_to_create.validation()?;

    let event = add_event(&event_to_create, &pool).await?;

    Ok(json_response(&event, StatusCode::CREATED))
}

async fn add_event(
    event_to_create: &RawEvent,
    pool: &DatabaseConnection,
) -> Result<event::Model, DbErr> {
    let now = Utc::now().naive_utc();

    let event = event::ActiveModel {
        id: ActiveValue::NotSet,
        name: ActiveValue::Set(event_to_create.name.clone()),
        link: ActiveValue::Set(event_to_create.link.clone()),
        max_adults: ActiveValue::Set(event_to_create.max_adults),
        max_children: ActiveValue::Set(event_to_create.max_children),
        max_adults_per_reservation: ActiveValue::Set(event_to_create.max_adults_per_reservation),
        max_children_per_reservation: ActiveValue::Set(
            event_to_create.max_children_per_reservation,
        ),
        ts: ActiveValue::Set(now),
        remind: ActiveValue::Set(event_to_create.remind.naive_utc()),
        state: ActiveValue::Set(EventState::default()),
        adult_ticket_price: ActiveValue::Set(event_to_create.adult_ticket_price),
        child_ticket_price: ActiveValue::Set(event_to_create.child_ticket_price),
        currency: ActiveValue::Set(event_to_create.currency.clone()),
    };

    let result = event::Entity::insert(event).exec(pool).await?;

    Ok(event::Entity::find_by_id(result.last_insert_id)
        .one(pool)
        .await?
        .unwrap())
}
