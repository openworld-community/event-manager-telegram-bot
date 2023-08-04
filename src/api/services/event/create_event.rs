use crate::api::services::event::types::RawEvent;
use crate::api::shared::{into_internal_server_error_response, QueryError};
use crate::api::utils::json_response;
use crate::db::mutate_event;
use crate::types::{DbPool, Event};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, Responder};
use tokio::task::spawn_blocking;

#[post("")]
pub async fn create_event(
    pool: Data<DbPool>,
    event_to_create: Json<RawEvent>,
) -> actix_web::Result<impl Responder> {
    event_to_create.validation()?;

    let mut event: Event = event_to_create.into_inner().into();
    let cloned = event.clone();
    let event_id = spawn_blocking(move || insert_event(&pool, &cloned))
        .await
        .map_err(into_internal_server_error_response)
        .await?;

    event.id = event_id;

    Ok(json_response(&event, StatusCode::CREATED))
}

async fn insert_event(pool: &DbPool, event: &Event) -> Result<i64, QueryError> {
    let conn = pool.get().await.unwrap();
    Ok(mutate_event(&conn, &event)
        .await
        .map_err(|e| QueryError::DatabaseQueryError(e.to_string()))?)
}
