use std::sync::Arc;
use sea_orm::DatabaseConnection;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Message, Requester};
use teloxide::{Bot, RequestError};
use teloxide::types::{InlineKeyboardButton, User};
use teloxide::utils::command::BotCommands;
use tracing::log::error;
use url::Url;
use entity::event::EventType;
use entity::new_types::EventState;
use crate::api::services::event::{event_list_stats, EventStats};
use crate::api::shared::RawPagination;
use crate::bot::Context;
use crate::bot::reply::ReplyMessage;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum UserCommand {
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

    let command = UserCommand::parse(message.text().unwrap(), &context.bot_name);

    match command {
        Ok(command) => {
            return commands_handler(&message, &bot, &command, &context).await;
        }
        Err(error) => {
            error!("parsing error: message {:?}   error {error}", message);
            let _ = bot.send_message(message.chat.id, format!("Ошибка разбора команды"))
                .reply_to_message_id(message.id)
                .await;
        }
    }

    Ok(())
}

async fn commands_handler(
    message: &Message,
    bot: &Bot,
    command: &UserCommand,
    context: &Context,
) -> Result<(), RequestError> {
    match command {
        UserCommand::Start => start_command_handler(bot, message, context).await,
    }
}


async fn start_command_handler(bot: &Bot, message: &Message, context: &Context) -> Result<(), RequestError> {
    let replay_message = event_list_message(&context.database_connection, message.from().unwrap()).await;


    replay_message.send(message, bot).await
}

const NO_EVENTS_HEADER: &str = "Нет мероприятий.";
const EVENTS_HEADER: &str = "Программа\nвремя / взросл.(детск.) места  / мероприятие\n<a href=\"{}\">инструкция</a> /donate";

async fn event_list_message(
    connection: &DatabaseConnection,
    user: &User
) -> ReplyMessage {
    let pagination = RawPagination {
        page: None,
        per_page: None,
    };
    let events = event_list_stats(user.id.0, &pagination, connection).await.unwrap();

    let header_message = if events.is_empty() {
        NO_EVENTS_HEADER
    } else {
        EVENTS_HEADER
    };


    ReplyMessage::new(header_message)
        .keyboard(build_events_keyboard(&events))
}

fn build_events_keyboard(events: &Vec<EventStats>) -> Vec<Vec<InlineKeyboardButton>> {
    events
        .into_iter()
        .map(|event| {
            match event.event_type() {
                EventType::Announcement => vec![announcement_button(event)],
                _ => vec![
                    InlineKeyboardButton::callback(callback_button_text(&event), "")
                ],
            }
        })
        .collect()
}

fn announcement_button(event: &EventStats) -> InlineKeyboardButton {
    InlineKeyboardButton::url(format!("ℹ️ {} {}", event.event_start_time, event.name), Url::parse(event.link.as_str()).unwrap())
}

fn callback_button_text(event: &EventStats) -> String {
    let emoji = emoji_for_event(event);
    let status = tickets_status(event);

    format!("{} {} / {} / {}", emoji, event.event_start_time, status, event.name)
}

fn tickets_status(event: &EventStats) -> String {
    if event.state == EventState::Open {
        if event.max_adults == 0 || event.max_children == 0 {
            (event.max_adults - event.my_wait_adults + event.max_children
                - event.my_wait_children)
                .to_string()
        } else {
            format!(
                "{}({})",
                event.max_adults - event.my_wait_adults,
                event.max_children - event.my_wait_children
            )
        }
    } else {
        "-".to_string()
    }
}

fn emoji_for_event(event: &EventStats) -> &'static str {
    if event.my_adults != 0 || event.my_children != 0 {
        "✅"
    } else if event.my_wait_adults != 0 || event.my_wait_children != 0 {
        "⏳"
    } else if event.event_type() != EventType::Paid {
        "✨" // todo: find a better emoji for free events
    } else {
        ""
    }
}
