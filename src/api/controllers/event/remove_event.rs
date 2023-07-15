use actix_web::{delete, HttpResponse, Responder};

#[delete("/{id}")]
pub async fn remove_event() -> actix_web::Result<impl Responder> {
    Ok(HttpResponse::NoContent())
}
