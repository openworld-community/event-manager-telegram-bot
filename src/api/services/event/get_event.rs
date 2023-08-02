use crate::api::services::event::db::select_event;
use crate::api::services::event::types::EventWithId;
use crate::api::shared::{into_internal_server_error_response, QueryError};
use crate::api::utils::json_response;
use crate::types::{DbPool, Event};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Path};
use actix_web::{get, Responder};
use tokio::task::spawn_blocking;

#[get("/{id}")]
pub async fn get_event(id: Path<i64>, pool: Data<DbPool>) -> actix_web::Result<impl Responder> {
    let event = spawn_blocking(move || perform_select_event(&pool.into_inner(), id.into_inner()))
        .await
        .map_err(into_internal_server_error_response)?
        .map_err(into_internal_server_error_response)?;

    Ok(json_response(&EventWithId::from(event), StatusCode::OK))
}

async fn perform_select_event(pool: &DbPool, id: i64) -> Result<Event, QueryError> {
    let conn = pool.get().await.unwrap();

    Ok(select_event(&conn, id)?)
}
