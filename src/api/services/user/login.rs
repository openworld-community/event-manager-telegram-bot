use crate::api::services::user::types::UserCred;
use crate::api::AppConfigData;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use jwt_simple::prelude::Claims;
use jwt_simple::prelude::Duration;
use jwt_simple::prelude::MACLike;
use serde_json::json;

#[post("/login")]
pub async fn login(
    user_cred: Json<UserCred>,
    app_config_data: Data<AppConfigData>,
) -> actix_web::Result<impl Responder> {
    if user_cred.into_inner() == app_config_data.admin_cred {
        let claims = Claims::create(Duration::from_hours(2));
        let token = app_config_data.jwt_key.authenticate(claims).unwrap();
        let val = json!({ "token": token });
        return Ok(HttpResponse::build(StatusCode::ACCEPTED).json(val));
    }
    Ok(HttpResponse::build(StatusCode::UNAUTHORIZED).body(""))
}
