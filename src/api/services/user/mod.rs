mod login;
mod types;

use crate::api::services::user::login::login as login_service;
use actix_web::{web, Scope};
pub use types::UserCred;

pub fn user_scope() -> Scope {
    web::scope("/user").service(login_service)
}
