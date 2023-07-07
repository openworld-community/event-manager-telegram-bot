use crate::api::shared::AppError;
use crate::api::utils::json_response;

use actix_web::http::StatusCode;
use actix_web::web::{Data, Path};
use actix_web::{get, Responder};

use crate::api::services::event;
use sea_orm::{DatabaseConnection, TransactionTrait};

#[get("/{id}")]
pub async fn event_by_id(
    id: Path<i32>,
    pool: Data<DatabaseConnection>,
) -> Result<impl Responder, AppError> {
    let event = event::get_event(&id.into_inner(), &pool.begin().await?)
        .await?
        .map_or(Err(AppError::NotFoundError), |val| Ok(val))?;

    Ok(json_response(&event, StatusCode::OK))
}
