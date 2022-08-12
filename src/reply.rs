use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
    RequestError,
};

/// Internal presentation for bot replies.
pub enum Reply {
    Message(ReplyMessage),
    Invoice {
        title: String,
        description: String,
        payload: String,
        currency: String,
        amount: u64,
    },
}
#[derive(Debug)]
pub struct ReplyMessage {
    pub message: String,
    pub parse_mode: ParseMode,
    pub disable_preview: bool,
    pub keyboard: Option<Vec<Vec<InlineKeyboardButton>>>,
}
impl ReplyMessage {
    pub fn new<T>(message: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            message: message.into(),
            parse_mode: ParseMode::Html,
            disable_preview: true,
            keyboard: None,
        }
    }

    pub fn text<T>(mut self, text: Option<T>) -> Self
    where
        T: Into<String>,
    {
        if let Some(text) = text {
            self.message.push_str(&text.into());
        }
        self
    }

    pub fn keyboard(mut self, mut keyboard: Vec<Vec<InlineKeyboardButton>>) -> Self {
        if keyboard.len() != 0 {
            match self.keyboard {
                Some(ref mut k) => k.append(&mut keyboard),
                None => {
                    self.keyboard = Some(keyboard);
                }
            }
        }
        self
    }

    pub fn pagination<T>(
        mut self,
        prev: T,
        next: T,
        participants: u64,
        limit: u64,
        offset: u64,
    ) -> anyhow::Result<Self>
    where
        T: serde::ser::Serialize,
    {
        if offset > 0 || participants == limit {
            let mut pagination: Vec<InlineKeyboardButton> = Vec::new();
            if offset > 0 {
                pagination.push(InlineKeyboardButton::callback(
                    "⬅️",
                    serde_json::to_string::<T>(&prev.into())?,
                ));
            }
            if participants == limit {
                pagination.push(InlineKeyboardButton::callback(
                    "➡️",
                    serde_json::to_string::<T>(&next.into())?,
                ));
            }
            match self.keyboard {
                Some(ref mut keyboard) => keyboard.push(pagination),
                None => {
                    self.keyboard = Some(vec![pagination]);
                }
            }
        }
        Ok(self)
    }

    pub fn parse_mode(mut self, parse_mode: ParseMode) -> Self {
        self.parse_mode = parse_mode;
        self
    }

    pub async fn send(self, msg: &Message, bot: &AutoSend<Bot>) -> Result<(), RequestError> {
        let fut = if let Some(keyboard) = self.keyboard {
            bot.send_message(msg.chat.id, self.message)
                .parse_mode(self.parse_mode)
                .disable_web_page_preview(self.disable_preview)
                .reply_markup(InlineKeyboardMarkup::new(keyboard))
        } else {
            bot.send_message(msg.chat.id, self.message)
                .parse_mode(self.parse_mode)
                .disable_web_page_preview(self.disable_preview)
        };
        fut.await.or_else(|e| {
            error!("Failed to send message to Telegram: {}", e);
            Err(e)
        })?;
        Ok(())
    }

    pub async fn edit(self, msg: &Message, bot: &AutoSend<Bot>) -> Result<(), RequestError> {
        let fut = if let Some(keyboard) = self.keyboard {
            bot.edit_message_text(msg.chat.id, msg.id, self.message)
                .parse_mode(self.parse_mode)
                .disable_web_page_preview(self.disable_preview)
                .reply_markup(InlineKeyboardMarkup::new(keyboard))
        } else {
            bot.edit_message_text(msg.chat.id, msg.id, self.message)
                .parse_mode(self.parse_mode)
                .disable_web_page_preview(self.disable_preview)
        };
        fut.await.or_else(|e| {
            error!("Failed to send message to Telegram: {}", e);
            Err(e)
        })?;
        Ok(())
    }
}

impl Into<Reply> for ReplyMessage {
    fn into(self) -> Reply {
        Reply::Message(self)
    }
}
