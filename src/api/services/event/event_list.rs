use crate::api::shared::{AppError, Pagination, RawPagination};
use crate::api::utils::json_response;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Query};
use actix_web::{get, Responder};
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, QuerySelect};
use entity::event;

#[get("")]
pub async fn event_list(
    pool: Data<DatabaseConnection>,
    params: Query<RawPagination>,
) -> Result<impl Responder, AppError> {
    let events = get_event_list(&params.into_inner(), &pool).await?;

    Ok(json_response(&events, StatusCode::OK))
}

async fn get_event_list(pagination: &impl Pagination, pool: &DatabaseConnection) -> Result<Vec<event::Model>, DbErr> {
    event::Entity::find()
        .limit(pagination.limit())
        .offset(pagination.offset())
        .all(pool)
        .await
}

