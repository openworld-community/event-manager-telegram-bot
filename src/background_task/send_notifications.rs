use crate::api::services::message::{get_pending_messages, MessageBatch};
use crate::api::services::message_sent::add_message_sent;
use crate::bot::callback_list::CallbackQuery;
use crate::configuration::config::Config;
use chrono::Utc;
use entity::new_types::MessageType;
use sea_orm::{DatabaseConnection, DbErr, TransactionTrait};
use serde_json::Error;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Requester, RequesterExt, UserId};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};
use teloxide::Bot;

fn is_mailing_time(cfg: &Config, current_time: &chrono::DateTime<Utc>) -> bool {
    let ts = current_time.timestamp();
    let seconds_from_midnight = ts % 86400;

    return seconds_from_midnight >= cfg.mailing_hours_from as i64
        && seconds_from_midnight < cfg.mailing_hours_to as i64;
}

pub async fn send_notifications(
    cfg: &Config,
    bot: &Bot,
    connection: &DatabaseConnection,
) -> Result<(usize, bool), DbErr> {
    let transaction = connection.begin().await?;

    let current_time = Utc::now();
    let mut notifications: usize = 0;
    let mut batch_contains_waiting_list_prompt = false;

    if !is_mailing_time(cfg, &current_time) {
        return Ok((notifications, batch_contains_waiting_list_prompt));
    }

    let messages = get_pending_messages(
        &current_time,
        cfg.limit_bulk_notifications_per_second as i32,
        &transaction,
    )
    .await
    .unwrap();

    for message in messages {
        notifications += message.recipients.len();
        let keyboard = one_inline_button_keyboard(link_to_event_button(&message).unwrap());
        for user in message.recipients {
            bot.send_message(UserId(user as u64), &message.text)
                .parse_mode(ParseMode::Html)
                .disable_web_page_preview(true)
                .reply_markup(keyboard.clone())
                .await;

            add_message_sent(message.message_id, user, &transaction)
                .await
                .unwrap();

            if message.message_type == MessageType::WaitingListPrompt {
                batch_contains_waiting_list_prompt = true;
            }
        }
    }

    transaction.commit().await?;

    return Ok((notifications, batch_contains_waiting_list_prompt));
}

fn link_to_event_button(message: &MessageBatch) -> Result<InlineKeyboardButton, Error> {
    let data = match message.is_paid() {
        true => serde_json::to_string(&CallbackQuery::PaidEvent {
            event_id: message.event_id as u64,
            adults: 0,
            children: 0,
            offset: 0,
        }),
        false => serde_json::to_string(&CallbackQuery::Event {
            event_id: message.event_id as u64,
            offset: 0,
        }),
    }?;

    let build_callback = InlineKeyboardButton::callback("К мероприятию", data);

    Ok(build_callback)
}

fn one_inline_button_keyboard(button: InlineKeyboardButton) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![button]])
}
