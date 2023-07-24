use crate::api::controllers::event::types::OptionalRawEvent;
use crate::api::shared::AppError;
use crate::api::utils::json_response;

use actix_web::http::StatusCode;
use actix_web::web::{Data, Json, Path};
use actix_web::{post, Responder};
use sea_orm::{DatabaseConnection, TransactionTrait};

use crate::api::services::event;

#[post("/{id}")]
pub async fn update_event(
    id: Path<i32>,
    event_to_update: Json<OptionalRawEvent>,
    pool: Data<DatabaseConnection>,
) -> Result<impl Responder, AppError> {
    let id = id.into_inner();
    let event_to_update = event_to_update.into_inner();

    let transaction = pool.begin().await?;

    let event = event::get_event(&id, &transaction)
        .await?
        .map_or(Err(AppError::NotFoundError), |val| Ok(val))?;

    event_to_update.validation(&event)?;

    let updated = event::update_event(&id, &event_to_update, &transaction).await?;

    transaction.commit().await?;

    Ok(json_response(&updated, StatusCode::OK))
}
