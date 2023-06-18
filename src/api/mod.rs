mod services;
mod shared;
mod utils;
mod middlewares;

use crate::api::services::event::event_scope;
use crate::types::DbPool;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use std::net::ToSocketAddrs;
use middlewares::cors_middleware;

pub fn setup_api_server<Addr: ToSocketAddrs>(addr: &Addr, con_pool: &DbPool) -> Server {
    let pool = con_pool.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(cors_middleware())
            .service(event_scope())
    })
    .bind(&addr)
    .expect("to bind on socket")
    .run()
}
