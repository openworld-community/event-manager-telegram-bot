use crate::bot::updates_handler::build_bot_handler;

use std::collections::HashSet;
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use teloxide::dispatching::{DefaultKey, DispatcherBuilder};
use teloxide::prelude::{Dispatcher, LoggingErrorHandler};
use teloxide::{Bot, dptree, RequestError};


mod admin_endpoint;
pub mod callback_list;
mod updates_handler;
mod user_endpoint;
mod reply;

pub struct Context {
    pub bot_name: String,
    pub admin_ids: HashSet<u64>,
    pub database_connection: DatabaseConnection,
}




pub async fn run_bot<'a>(
    bot: &Bot,
    context: Arc<Context>
) {
    Dispatcher::builder(bot.clone(), build_bot_handler())
        .enable_ctrlc_handler()
        .error_handler(LoggingErrorHandler::new())
        .dependencies(dptree::deps![context])
        .build()
        .dispatch()
        .await
}
