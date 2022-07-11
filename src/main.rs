#[macro_use]
extern crate serde;
#[macro_use]
extern crate num_derive;
extern crate num;
use chrono::DateTime;
use futures::StreamExt;
use std::collections::HashSet;
use std::{fs::File, io::prelude::*, time::Duration};
use telegram_bot::{Api, CanAnswerCallbackQuery, CanSendMessage, UpdateKind};
#[macro_use]
extern crate log;

mod admin_message_handler;
mod db;
mod message_handler;
mod types;
mod util;

use admin_message_handler::AdminMessageHandler;
use db::EventDB;
use message_handler::MessageHandler;
use types::{Configuration, DialogState, User};
use util::{format_ts, get_unix_time};

use crate::types::MessageType;

#[tokio::main]
async fn main() -> std::result::Result<(), String> {
    env_logger::init();
    let matches = clap::App::new("event-manager-telegram-bot")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("event-manager-telegram-bot")
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .help("Configuration file")
                .takes_value(true)
                .default_value(""),
        )
        .get_matches();

    let config = matches.value_of("config").unwrap();
    let mut f = File::open(config).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    let mut config = toml::from_str::<Configuration>(&contents)
        .map_err(|e| format!("Error loading configuration: {}", e))
        .unwrap();

    parse_config(&mut config).unwrap();

    let admins = config
        .admin_ids
        .split(',')
        .into_iter()
        .filter_map(|id| id.parse::<i64>().ok())
        .collect();

    let api = Api::new(config.telegram_bot_token.clone());
    let db = db::EventDB::open("./events.db3")
        .map_err(|e| format!("Failed to init db: {}", e))
        .unwrap();

    let message_handler = MessageHandler::new(&db, &api, &config);
    let admin_handler = AdminMessageHandler::new(&db, &api, &config, &message_handler);
    let mut stream = api.stream();
    let mut dialog_state = DialogState::new(1_000_000);

    let mut next_break = tokio::time::Instant::now() + Duration::from_millis(1000);
    loop {
        match tokio::time::timeout_at(next_break, stream.next()).await {
            Ok(update) => {
                if let Some(update) = update {
                    let update = match update {
                        Ok(v) => v,
                        Err(e) => {
                            error!("Failed to parse update: {}", e);
                            continue;
                        }
                    };
                    if let UpdateKind::Message(msg) = update.kind {
                        //debug!("message: {:?}", &msg);
                        if msg.from.is_bot {
                            warn!("Bot ignored");
                            continue;
                        }
                        let user = User::new(msg.from, &admins);
                        match &msg.kind {
                            telegram_bot::types::MessageKind::Text { data, .. } => {
                                //debug!("Text: {}", data);
                                if !user.is_admin
                                    || !admin_handler.process_message(&user, data, &admins)
                                {
                                    message_handler.process_message(&user, data, &mut dialog_state);
                                }
                            }
                            _ => {
                                error!("Failed to parse message.");
                            }
                        }
                    } else if let UpdateKind::CallbackQuery(msg) = update.kind {
                        //debug!("callback: {:?}", &msg);
                        api.spawn(msg.acknowledge());
                        let user = User::new(msg.from, &admins);
                        match msg.data {
                            Some(data) => {
                                if !user.is_admin
                                    || !admin_handler.process_query(&user, &data, &msg.message)
                                {
                                    message_handler.process_query(
                                        &user,
                                        &data,
                                        &msg.message,
                                        &mut dialog_state,
                                    );
                                }
                            }
                            None => {
                                error!("Failed to parse callback.");
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Timeout elapsed?
                next_break = tokio::time::Instant::now()
                    + Duration::from_millis(
                        if perform_bulk_tasks(&db, &api, &config, &admins).await {
                            1000
                        } else {
                            30000
                        },
                    )
            }
        }
    }
}

async fn perform_bulk_tasks(
    db: &EventDB,
    api: &Api,
    config: &Configuration,
    admins: &HashSet<i64>,
) -> bool {
    let mut notifications = 0;
    let mut batch_contains_waiting_list_prompt = false;
    let ts = get_unix_time();
    let num_seconds_from_midnight = ts % 86400;

    if num_seconds_from_midnight >= config.mailing_hours_from.unwrap()
        && num_seconds_from_midnight < config.mailing_hours_to.unwrap()
    {
        match db.get_pending_messages(ts, config.limit_bulk_notifications_per_second) {
            Ok(messages) => {
                for m in messages {
                    notifications += m.recipients.len();
                    let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                    let v = vec![telegram_bot::types::InlineKeyboardButton::callback(
                        "К мероприятию",
                        format!("event {} 0", m.event_id),
                    )];
                    keyboard.add_row(v);
                    for u in m.recipients {
                        debug!("Sending notification {} to {} {}", m.message_id, u, &m.text);
                        match api
                            .send_timeout(
                                telegram_bot::types::UserId::new(u)
                                    .text(&m.text)
                                    .parse_mode(telegram_bot::types::ParseMode::Html)
                                    .disable_preview()
                                    .reply_markup(keyboard.clone()),
                                Duration::from_secs(2),
                            )
                            .await
                        {
                            Ok(result) => {
                                if result.is_none() {
                                    return false; // time-out - retry later
                                }
                            }
                            Err(e) => {
                                // don't retry - user might have blocked the bot
                                error!("Failed to deliver notification: {}", e);
                            }
                        }
                        if let Err(e) = db.save_receipt(m.message_id, u) {
                            error!("Failed to save receipt: {}", e);
                        }
                        if m.message_type == MessageType::WaitingListPrompt {
                            batch_contains_waiting_list_prompt = true;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to get pending messages: {}", e);
            }
        }
    }

    if config.cleanup_old_events {
        // Clean up.
        if db
            .clear_old_events(
                ts - config.drop_events_after_hours * 60 * 60,
                config.automatic_blacklisting,
                config.cancel_future_reservations_on_ban,
                &admins,
            )
            .is_ok()
            == false
        {
            error!("Failed to clear old events at {}", ts);
        }

        // Age black lists.
        if db
            .clear_black_list(ts - config.delete_from_black_list_after_days * 24 * 60 * 60)
            .is_ok()
            == false
        {
            error!("Failed to clear black list at {}", ts);
        }
    }
    notifications > 0 && batch_contains_waiting_list_prompt == false
}

fn parse_config(config: &mut Configuration) -> Result<(), String> {
    let parts: Vec<&str> = config.mailing_hours.split('.').collect();
    if parts.len() != 3 {
        return Err("Wrong mailing hours format.".to_string());
    }
    match (
        DateTime::parse_from_str(&format!("2022-07-06 {}", parts[0]), "%Y-%m-%d %H:%M  %z"),
        DateTime::parse_from_str(&format!("2022-07-06 {}", parts[2]), "%Y-%m-%d %H:%M  %z"),
    ) {
        (Ok(from), Ok(to)) => {
            config.mailing_hours_from = Some(from.timestamp() % 86400);
            config.mailing_hours_to = Some(to.timestamp() % 86400);
            Ok(())
        }
        _ => Err("Failed to farse mailing hours.".to_string()),
    }
}
