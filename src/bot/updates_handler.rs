use crate::bot::user_endpoint;
use crate::bot::{admin_endpoint, Context};
use std::sync::Arc;
use teloxide::dispatching::{DpHandlerDescription, UpdateFilterExt};
use teloxide::dptree::Handler;
use teloxide::prelude::{dptree, DependencyMap, Requester, Update};
use teloxide::types::User;
use teloxide::{Bot, RequestError};

pub fn build_bot_handler(
) -> Handler<'static, DependencyMap, Result<(), RequestError>, DpHandlerDescription> {
    dptree::entry()
        .branch(
            Update::filter_message()
                .chain(dptree::filter_map(user_filter))
                .branch(dptree::filter(is_admin).endpoint(admin_endpoint::handler))
                .endpoint(user_endpoint::handler),
        )
        .endpoint(default_handler)
}

async fn default_handler(update: Update, bot: &Bot) -> Result<(), RequestError> {
    match update.chat() {
        None => Ok(()),
        Some(chat) => {
            bot.send_message(chat.id, "sadsad").await?;
            Ok(())
        }
    }
}

pub fn user_filter(update: Update) -> Option<User> {
    match update.user() {
        None => None,
        Some(user) => Some(user.clone()),
    }
}

fn is_admin(user: User, context: Arc<Context>) -> bool {
    context.admin_ids.contains(&user.id.0)
}
