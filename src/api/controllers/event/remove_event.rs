use actix_web::{delete, HttpResponse, Responder};
use actix_web::web::{Data, Path};
use sea_orm::{DatabaseConnection, TransactionTrait};
use crate::api::services::event::remove_event;
use crate::api::shared::AppError;

#[delete("/{id}")]
pub async fn remove_event_handler(
    id: Path<i32>,
    pool: Data<DatabaseConnection>,
) -> Result<impl Responder, AppError> {
    let transaction = pool.begin().await?;

    remove_event(&id, &transaction).await?;

    transaction.commit().await?;

    Ok(HttpResponse::NoContent())
}
