use crate::api::shared::{into_internal_server_error_response, RawPagination};
use crate::api::utils::json_response;

use crate::types::DbPool;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Query};
use actix_web::{get, Responder};

use crate::api::services::event::db;
use tokio::task::spawn_blocking;

#[get("")]
pub async fn event_list(
    pool: Data<DbPool>,
    params: Query<RawPagination>,
) -> actix_web::Result<impl Responder> {
    let events = spawn_blocking(move || db::get_event_list(&pool, &params.into_inner().into()))
        .await
        .map_err(into_internal_server_error_response)?
        .map_err(into_internal_server_error_response);

    Ok(json_response(&events, StatusCode::OK))
}
