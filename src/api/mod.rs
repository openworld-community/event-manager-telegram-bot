mod controllers;
mod middlewares;
mod services;
mod shared;
mod utils;

use crate::api::middlewares::LogTime;
use crate::api::controllers::event::event_scope;
use crate::api::services::event::event_scope;
use crate::types::DbPool;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use middlewares::cors_middleware;
use sea_orm::DatabaseConnection;
use std::net::ToSocketAddrs;
use tracing_actix_web::TracingLogger;

pub fn setup_api_server<Addr: ToSocketAddrs>(addr: &Addr, con_pool: &DatabaseConnection) -> Server {
    let pool = con_pool.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(cors_middleware())
            .wrap(LogTime::new())
            .wrap(TracingLogger::default())
            .service(event_scope())
    })
    .bind(&addr)
    .expect("to bind on socket")
    .run()
}
