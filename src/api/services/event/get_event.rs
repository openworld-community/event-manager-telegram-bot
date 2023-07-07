

use crate::api::shared::{AppError};
use crate::api::utils::json_response;

use actix_web::http::StatusCode;
use actix_web::web::{Data, Path};
use actix_web::{get, Responder};

use sea_orm::{DatabaseConnection, DbErr, EntityTrait};
use entity::event;
use entity::event::Model;

#[get("/{id}")]
pub async fn event_by_id(id: Path<i32>, pool: Data<DatabaseConnection>) -> Result<impl Responder, AppError> {
    let event = get_event(id.into_inner(), &pool)
        .await?
        .map_or(Err(AppError::NotFoundError), |val| Ok(val))?;

    Ok(json_response(&event, StatusCode::OK))
}

async fn get_event(id: i32, pool: &DatabaseConnection) -> Result<Option<Model>, DbErr> {
    event::Entity::find_by_id(id).one(pool).await
}
