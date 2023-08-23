mod api;
mod app_errors;
mod background_task;
mod bot;
mod configuration;
mod set_up_logger;
mod util;

use std::sync::Arc;
use crate::api::setup_api_server;




use migration::{Migrator, MigratorTrait};

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use teloxide::prelude::{Bot, Requester};
use tokio::{join};

use crate::app_errors::AppErrors;
use crate::background_task::perform_background_task;
use crate::bot::{Context, run_bot};
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
    let me = bot.get_me()
        .await
        .map_err(|err| AppErrors::ErrorToGetBotName(err))?;

    let context = Arc::new(Context {
        bot_name: me.username().to_string(),
        admin_ids: config.admins.clone(),
    });


    let _ = join!(
        setup_api_server(
        &config.api_socket_address,
        &database_connection,
    ),
        perform_background_task(
        &bot,
        &config,
        &database_connection,
    ),
       run_bot(&bot, context)
    );

    return Ok(());
}

