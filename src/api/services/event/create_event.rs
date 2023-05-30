use crate::api::services::event::types::RawEvent;
use crate::api::shared::{into_internal_server_error_responce, QueryError};
use crate::api::utils::json_responce;
use crate::db::mutate_event;
use crate::types::{DbPool, Event};
use crate::admin_message_handler::add_event;
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
    let event_id = spawn_blocking(move || add_event(&pool, &cloned.to_string()))
        .await
        .map_err(into_internal_server_error_responce)?
        .map_err(into_internal_server_error_responce)?;

    event.id = event_id;

    Ok(json_responce(&event, StatusCode::CREATED))
}
//
// fn insert_event(pool: &DbPool, event: &Event) -> Result<u64, QueryError> {
//     let con = pool.get()?;
//     Ok(mutate_event(&con, &event)?)
// }
