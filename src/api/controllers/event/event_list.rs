use crate::api::services::event;
use crate::api::shared::{AppError, RawPagination};
use crate::api::utils::json_response;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Query};
use actix_web::{get, Responder};
use sea_orm::DatabaseConnection;

#[get("")]
pub async fn event_list(
    pool: Data<DatabaseConnection>,
    params: Query<RawPagination>,
) -> Result<impl Responder, AppError> {
    let events = event::event_list(&params.into_inner(), &pool).await?;

    Ok(json_response(&events, StatusCode::OK))
}
