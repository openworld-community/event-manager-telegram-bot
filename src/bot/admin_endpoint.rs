use std::sync::Arc;
use crate::bot::Context;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Message, Requester};
use teloxide::utils::command::BotCommands;
use teloxide::{Bot, RequestError};

#[derive(BotCommands, Clone)]
enum AdminCommand {
    Start,
}

async fn send_unsupported_message_error(bot: &Bot, message: &Message) -> Result<(), RequestError> {
    bot.send_message(message.chat.id, "Это сообщение не поддерживается")
        .reply_to_message_id(message.id)
        .await?;
    Ok(())
}

pub async fn handler(message: Message, bot: Bot, context: Arc<Context>) -> Result<(), RequestError> {
    if message.text().is_none() {
        return send_unsupported_message_error(&bot, &message).await;
    }

    let command = AdminCommand::parse(message.text().unwrap(), &context.bot_name);

    match command {
        Ok(command) => {
            return commands_handler(&message, &bot, &command).await;
        }
        Err(error) => {
            bot.send_message(message.chat.id, format!("Ошибка разбора команды {error}"))
                .reply_to_message_id(message.id)
                .await;
        }
    }

    Ok(())
}

async fn commands_handler(
    message: &Message,
    bot: &Bot,
    command: &AdminCommand,
) -> Result<(), RequestError> {
    match command {
        AdminCommand::Start => start_command_handler(bot, message).await,
    }
}

async fn start_command_handler(bot: &Bot, message: &Message) -> Result<(), RequestError> {
    bot.send_message(message.chat.id, "Здравствуй админ")
        .await?;
    Ok(())
}
