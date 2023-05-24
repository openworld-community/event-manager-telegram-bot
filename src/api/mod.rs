mod services;
mod shared;

use crate::api::services::event::event_scope;
use crate::types::DbPool;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use std::net::ToSocketAddrs;

pub fn setup_api_server<Addr: ToSocketAddrs>(addr: &Addr, con_pool: DbPool) -> Server {
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(con_pool.clone()))
            .service(event_scope())
    })
    .bind(&addr)
    .expect("to bind on socket")
    .run()
}
