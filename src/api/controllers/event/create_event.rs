use crate::api::controllers::event::types::RawEvent;
use crate::api::shared::AppError;
use crate::api::utils::json_response;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, Responder};
use sea_orm::{ActiveValue, DatabaseConnection, DbErr, EntityTrait};
use crate::api::services::event;

#[post("")]
pub async fn create_event(
    pool: Data<DatabaseConnection>,
    event_to_create: Json<RawEvent>,
) -> Result<impl Responder, AppError> {
    event_to_create.validation()?;

    let event = event::create_event(&event_to_create, &pool).await?;

    Ok(json_response(&event, StatusCode::CREATED))
}

