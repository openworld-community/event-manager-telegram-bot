mod middlewares;
mod services;
mod shared;
mod utils;

use crate::api::services::event::event_scope;
use crate::types::DbPool;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use middlewares::cors_middleware;
use std::net::ToSocketAddrs;

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
