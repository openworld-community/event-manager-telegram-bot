use crate::types::DbPool;
use actix_web::web::Data;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Scope};
use rusqlite::params;
use tokio::task::spawn_blocking;

pub fn event_scope() -> Scope {
    web::scope("/event").service(index)
}

#[get("")]
async fn index(pool: Data<DbPool>) -> impl Responder {
    let timestamp = spawn_blocking(move || {
        let con = pool.get().unwrap();
        let value: String = con
            .query_row("SELECT DATE();", params![], |row| row.get(0))
            .unwrap();
        value
    })
    .await
    .expect("spawn_block error");
    HttpResponse::Ok().body(format!("current date from database {}", timestamp))
}
