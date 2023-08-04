use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use serde::Serialize;

pub fn json_response<Entity: Serialize>(entity: &Entity, status: StatusCode) -> HttpResponse {
    HttpResponse::build(status)
        .insert_header(ContentType::json())
        .body(serde_json::to_string(entity).unwrap())
}
