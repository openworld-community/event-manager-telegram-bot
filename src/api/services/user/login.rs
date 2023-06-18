use crate::api::services::user::types::UserCred;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};

#[post("/login")]
pub async fn login(
    user_cred: Json<UserCred>,
    admin_cred: Data<UserCred>,
) -> actix_web::Result<impl Responder> {
    if user_cred == admin_cred {
        Ok(HttpResponse::build(StatusCode::NO_CONTENT))
    }
    Ok(HttpResponse::build(StatusCode::UNAUTHORIZED))
}
