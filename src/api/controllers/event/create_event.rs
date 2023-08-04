use crate::api::controllers::event::types::RawEvent;
use crate::api::services::event;
use crate::api::shared::json_response;
use crate::api::shared::AppError;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, Responder};
use sea_orm::{DatabaseConnection, TransactionTrait};

#[post("")]
pub async fn create_event(
    pool: Data<DatabaseConnection>,
    event_to_create: Json<RawEvent>,
) -> Result<impl Responder, AppError> {
    event_to_create.validation()?;

    let transaction = pool.begin().await?;

    let create_event_result = event::create_event(&event_to_create, &transaction).await?;

    transaction.commit().await?;

    Ok(json_response(
        &create_event_result.event,
        StatusCode::CREATED,
    ))
}
