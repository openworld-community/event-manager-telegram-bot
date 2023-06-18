mod services;
mod shared;
mod utils;

use crate::types::DbPool;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
pub use services::UserCred;
use services::{event_scope, user_scope};
use std::net::ToSocketAddrs;

pub struct ApiServerConfig<'a, Addr: ToSocketAddrs> {
    pub addr: &'a Addr,
    pub con_poll: &'a DbPool,
    pub admin_cred: &'a UserCred,
}

pub fn setup_api_server<Addr: ToSocketAddrs>(config: ApiServerConfig<Addr>) -> Server {
    let pool = config.con_poll.clone();
    let admin_cred = config.admin_cred;
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(admin_cred))
            .app_data(web::Data::new(pool.clone()))
            .service(event_scope())
            .service(user_scope())
    })
    .bind(&config.addr)
    .expect("to bind on socket")
    .run()
}
