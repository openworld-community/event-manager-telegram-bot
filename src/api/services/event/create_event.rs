use crate::types::{DbPool, Event};
use std::fmt::Debug;

use crate::db::mutate_event;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, Responder};

use crate::api::shared::{into_internal_server_error_responce, QueryError};
use crate::api::utils::{json_responce, validation_error_to_http};
use chrono::{DateTime, Utc};
use tokio::task::spawn_blocking;
use crate::api::services::event::types::RawEvent;


#[post("")]
pub async fn create_event(
    pool: Data<DbPool>,
    event_to_create: Json<RawEvent>,
) -> actix_web::Result<impl Responder> {
    event_to_create.validation()?;

    let mut event: Event = event_to_create.into_inner().into();
    let cloned = event.clone();
    let event_id = spawn_blocking(move || insert_event(&pool, &cloned)).await;

    event.id = event_id
        .map_err(into_internal_server_error_responce)?
        .map_err(into_internal_server_error_responce)?;

    Ok(json_responce(&event, StatusCode::CREATED))
}

fn insert_event(pool: &DbPool, event: &Event) -> Result<u64, QueryError> {
    let con = pool.get()?;
    Ok(mutate_event(&con, &event)?)
}
