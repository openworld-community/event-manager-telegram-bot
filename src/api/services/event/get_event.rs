use crate::api::services::event::db::select_event;
use crate::api::services::event::types::EventWithId;
use crate::api::shared::{AppError, QueryError};
use crate::api::utils::json_response;
use crate::types::{DbPool, Event};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Path};
use actix_web::{get, Responder};
use actix_web::error::ErrorBadRequest;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait};
use entity::event;
use entity::event::Model;

#[get("/{id}")]
pub async fn event_by_id(id: Path<i64>, pool: Data<DatabaseConnection>) -> Result<impl Responder, AppError> {
    let event = get_event(id.into_inner(), &pool)
        .await?
        .map_or(Err(AppError::NotFoundError("Event")), |val| Ok(val))?;

    Ok(json_response(&event, StatusCode::OK))
}

async fn get_event(id: i64, pool: &DatabaseConnection) -> Result<Option<Model>, DbErr> {
    event::Entity::find_by_id(id).one(pool).await
}
