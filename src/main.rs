mod api;
mod app_errors;
mod background_task;
mod bot;
mod configuration;
mod set_up_logger;
mod util;

use crate::api::setup_api_server;

use migration::{Migrator, MigratorTrait};

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use teloxide::prelude::Bot;

use crate::app_errors::AppErrors;
use crate::background_task::perform_background_task;
use crate::configuration::get_config;
use crate::set_up_logger::set_up_logger;

async fn build_connection(database_connection: &String) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(database_connection.clone());
    opt.sqlx_logging(true);
    Database::connect(opt).await
}

#[tokio::main]
async fn main() -> Result<(), AppErrors> {
    set_up_logger();

    let config = get_config();

    let database_connection = build_connection(&config.database_connection).await?;
    Migrator::up(&database_connection, None).await?;

    let bot = Bot::new(&config.telegram_bot_token);

    tokio::spawn(setup_api_server(
        &config.api_socket_address,
        &database_connection,
    ));

    perform_background_task(&bot, &config, &database_connection).await;

    return Ok(());
}
