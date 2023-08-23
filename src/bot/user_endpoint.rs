use teloxide::prelude::{Message, Requester};
use teloxide::{Bot, RequestError};

pub async fn handler(bot: Bot, message: Message) -> Result<(), RequestError> {
    bot.send_message(message.chat.id, "Привет клиент").await?;

    Ok(())
}
