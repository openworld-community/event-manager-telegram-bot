use std::fmt::{Debug, Display, Formatter};
use crate::api::services::user::types::UserCred;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder, ResponseError};
use actix_web::Error;


#[derive(Debug)]
struct InvalidConditionals {}

impl Display for InvalidConditionals {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid credentials")
    }
}

impl ResponseError for InvalidConditionals {
    fn status_code(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}


#[post("/login")]
pub async fn login(user_cred: Json<UserCred>, admin_cred: Data<UserCred>) -> actix_web::Result<impl Responder> {
    if user_cred == admin_cred {
        Ok(HttpResponse::build(StatusCode::NO_CONTENT))
    }
    Err(Error::from(InvalidConditionals {}))
}
