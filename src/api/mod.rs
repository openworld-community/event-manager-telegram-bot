mod event_scope;
mod shared;

use crate::api::event_scope::event_scope;
use crate::types::DbPool;
use actix_web::dev::Server;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
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
