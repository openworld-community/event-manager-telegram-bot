#[macro_use]
extern crate serde;
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

    let config = toml::from_str::<Configuration>(&contents)
        .map_err(|e| format!("Error loading configuration: {}", e))
        .unwrap();

    let mut admins: HashSet<i64> = HashSet::new();
    let ids: Vec<&str> = config.admin_ids.split(',').collect();
    for id in &ids {
        if let Ok(v) = id.parse::<i64>() {
            admins.insert(v);
        }
    }

    let api = Api::new(config.telegram_bot_token.clone());
    let db = db::EventDB::open("./events.db3")
        .map_err(|e| format!("Failed to init db: {}", e))
        .unwrap();

    let message_handler = MessageHandler::new(&db, &api, &config);
    let admin_handler = AdminMessageHandler::new(&db, &api, &config, &message_handler);
    let mut stream = api.stream();
    let mut dialog_state = DialogState::new(1_000_000);

    let mut next_break = tokio::time::Instant::now() + Duration::from_millis(6000);
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
                        debug!("message: {:?}", &msg);
                        if msg.from.is_bot {
                            warn!("Bot ignored");
                            continue;
                        }
                        let user = User::new(msg.from, &admins);
                        match &msg.kind {
                            telegram_bot::types::MessageKind::Text { data, .. } => {
                                debug!("Text: {}", data);
                                if !user.is_admin
                                    || !admin_handler.process_message(&user, data, &admins).await
                                {
                                    message_handler
                                        .process_message(&user, data, &mut dialog_state);
                                }
                            }
                            _ => {
                                error!("Failed to parse message.");
                            }
                        }
                    } else if let UpdateKind::CallbackQuery(msg) = update.kind {
                        debug!("callback: {:?}", &msg);
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
                if config.perform_periodic_tasks {
                    perform_periodic_tasks(&db, &api, &config, &admins).await;
                }
                next_break = tokio::time::Instant::now() + Duration::from_millis(60000);
            }
        }
    }
}

async fn perform_periodic_tasks(
    db: &EventDB,
    api: &Api,
    config: &Configuration,
    admins: &HashSet<i64>,
) {
    // Time to send out reminders?
    let ts = get_unix_time();
    match db.get_user_reminders(ts) {
        Ok(reminders) => {
            for s in &reminders {
                let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                v.push(telegram_bot::types::InlineKeyboardButton::callback(
                    "Отменить моё участие",
                    format!("wontgo {}", s.event_id),
                ));
                keyboard.add_row(v);
                debug!("sending reminder");
                api.spawn(
                    telegram_bot::types::UserId::new(s.user_id).text(
                        format!("\nЗдравствуйте!\nНе забудьте, пожалуйста, что вы записались на\n<a href=\"{}\">{}</a>\
                        \nНачало: {}\n\
                        <b>ВНИМАНИЕ!</b> Если вы не сможете прийти и не отмените бронь заблаговременно, то не сможете больше бронировать. Извините, но бесплатные билеты не должны пропадать.\n",
                        s.link, s.name, format_ts(s.ts), )
                    )
                    .parse_mode(telegram_bot::types::ParseMode::Html)
                    .disable_preview()
                    .reply_markup(keyboard),
                );
                tokio::time::sleep(Duration::from_millis(40)).await;
                // not to hit the limits
            }
            if db.clear_user_reminders(ts).is_ok() == false {
                error!("Failed to clear reminders at {}", ts);
            }
        }
        Err(_e) => {
            error!("Failed to get reminders at {}", ts);
        }
    }

    // Clean up.
    if db
        .clear_old_events(
            ts - config.drop_events_after_hours * 60 * 60,
            config.automatic_blacklisting,
            &admins,
        )
        .is_ok()
        == false
    {
        error!("Failed to clear old events at {}", ts);
    }

    // Process black lists.
    if db
        .clear_black_list(ts - config.delete_from_black_list_after_days * 24 * 60 * 60)
        .is_ok()
        == false
    {
        error!("Failed to clear black list at {}", ts);
    }
}
