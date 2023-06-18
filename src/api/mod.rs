mod middlewares;
mod services;
mod shared;
mod utils;

use crate::api::middlewares::auth_middleware;
use crate::configuration::config::JwtKeyAlgorithm;
use crate::format::header;
use crate::types::DbPool;
use actix_web::dev::{Server, Service};
use actix_web::http::header;
use actix_web::{web, App, HttpResponse, HttpServer};
pub use services::UserCred;
use services::{event_scope, user_scope};
use std::net::ToSocketAddrs;

#[derive(Clone)]
pub struct AppConfigData {
    pub admin_cred: UserCred,
    pub jwt_key: JwtKeyAlgorithm,
}

pub fn setup_api_server<Addr: ToSocketAddrs>(
    addr: &Addr,
    con_poll: &DbPool,
    app_config_data: AppConfigData,
) -> Server {
    let pool = con_poll.clone();
    HttpServer::new(move || {
        let clonned_data = app_config_data.clone();
        let auth = auth_middleware(&clonned_data.jwt_key);
        App::new()
            .app_data(web::Data::new(clonned_data))
            .app_data(web::Data::new(pool.clone()))
            .service(event_scope().wrap(auth))
            .service(user_scope())
    })
    .bind(addr)
    .expect("to bind on socket")
    .run()
}
