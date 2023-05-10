#[macro_use]
extern crate serde;
#[macro_use]
extern crate num_derive;
extern crate num;
use std::collections::HashSet;
use std::sync::Arc;
use std::{fs::File, io::prelude::*, time::Duration};
use std::env;
use tokio::sync::Mutex;
#[macro_use]
extern crate log;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rusqlite;
use teloxide::{
    prelude::*,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup, LabeledPrice, MessageKind,
        MessageSuccessfulPayment, ParseMode, PreCheckoutQuery, Update, UserId,
    },
    RequestError,
};

mod admin_message_handler;
mod db;
mod format;
mod message_handler;
mod payments;
mod reply;
mod types;
mod util;

use crate::reply::*;
use crate::types::MessageType;
use r2d2_sqlite::SqliteConnectionManager;
use types::{Configuration, Context};
use util::get_unix_time;

#[tokio::main]
async fn main() {
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

    config.parse().unwrap();

    let admins: HashSet<u64> = config
        .admin_ids
        .split(',')
        .into_iter()
        .filter_map(|id| id.parse::<u64>().ok())
        .collect();

    let manager = SqliteConnectionManager::file("/data/events.db3");
    let pool = r2d2::Pool::new(manager).unwrap();
    if let Ok(conn) = pool.get() {
        db::create(&conn).expect("Failed to create db.");
    }

    let bot = Bot::new(&config.telegram_bot_token).auto_send();

    let bot_info = bot.get_me().await.unwrap();

    let bot_name = bot_info.user.username.unwrap_or("default_bot_name".to_string());

    env::set_var("BOT_NAME", bot_name);

    let context = Arc::new(Context {
        config,
        pool,
        admins,
        sign_up_mutex: Arc::new(Mutex::new(0u64)),
    });

    tokio::spawn(perform_bulk_tasks(bot.clone(), context.clone()));

    let handler = dptree::entry()
        .branch(Update::filter_pre_checkout_query().endpoint(pre_checkout_handler))
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![context])
        .default_handler(|upd| async move {
            log::warn!("Unhandled update: {:?}", upd);
        })
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}

async fn message_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    context: Arc<Context>,
) -> Result<(), RequestError> {
    trace!("received {:?}", msg);

    match &msg.kind {
        MessageKind::Common(_) => {
            if let Some(text) = msg.text() {
                if let Some(user) = msg.from() {
                    if user.is_bot {
                        warn!("Bot ignored");
                        return Ok(());
                    }
                    trace!("received {:?}", msg);
                    let u = crate::types::User::new(&user, &context.admins);
                    if let Ok(conn) = context.pool.get() {
                        let reply = if u.is_admin {
                            crate::admin_message_handler::handle_message(&conn, &u, text, &context)
                        } else {
                            crate::message_handler::handle_message(&conn, &u, text, &context)
                        };
                        match reply {
                            Ok(reply) => match reply {
                                Reply::Message(r) => {
                                    r.send(&msg, &bot).await?;
                                }
                                Reply::Invoice {
                                    title,
                                    description,
                                    payload,
                                    currency,
                                    amount,
                                } => {
                                    match bot
                                        .send_invoice(
                                            msg.chat.id,
                                            title,
                                            description,
                                            payload,
                                            &context.config.payment_provider_token,
                                            &currency,
                                            vec![LabeledPrice {
                                                label: currency.to_owned(),
                                                amount: amount as i32,
                                            }],
                                        )
                                        .need_name(false)
                                        .await
                                    {
                                        Ok(_) => {}
                                        Err(e) => {
                                            error!("failed to send invoice {:?}", e);
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                error!("Error in reply: {}", e);
                                bot.send_message(msg.chat.id, e.to_string()).await?;
                            }
                        }
                    }
                }
            }
        }
        MessageKind::SuccessfulPayment(MessageSuccessfulPayment { successful_payment }) => {
            trace!("successful_payment {:?}", &successful_payment);
            if let Ok(conn) = context.pool.get() {
                let res = crate::payments::checkout(&conn, successful_payment, &context);
                if let Err(e) = res {
                    error!("Failed to check out: {}", e);
                    bot.send_message(msg.chat.id, e.to_string()).await?;
                }
            }
        }
        _ => {
            warn!("unknown message");
        }
    }
    Ok(())
}

async fn callback_handler(
    q: CallbackQuery,
    bot: AutoSend<Bot>,
    context: Arc<Context>,
) -> Result<(), RequestError> {
    match (q.message, q.data) {
        (Some(msg), Some(data)) => {
            trace!("received {:?} {:?}", &msg, &data);
            let u = crate::types::User::new(&q.from, &context.admins);
            let mut lock;
            if data.starts_with("sign_up ") {
                lock = context.sign_up_mutex.lock().await;
                *lock = *lock + 1;
                // todo: use event based locking
            }
            if let Ok(conn) = context.pool.get() {
                let reply = if u.is_admin {
                    crate::admin_message_handler::handle_callback(&conn, &u, &data, &context)
                } else {
                    crate::message_handler::handle_callback(&conn, &u, &data, &context)
                };
                match reply {
                    Ok(reply) => match reply {
                        Reply::Message(r) => {
                            trace!("reply {:?}", r);
                            r.edit(&msg, &bot).await?;
                        }
                        Reply::Invoice {
                            title,
                            description,
                            payload,
                            currency,
                            amount,
                        } => {
                            match bot
                                .send_invoice(
                                    msg.chat.id,
                                    title,
                                    description,
                                    payload,
                                    &context.config.payment_provider_token,
                                    &currency,
                                    vec![LabeledPrice {
                                        label: currency.to_owned(),
                                        amount: amount as i32,
                                    }],
                                )
                                .need_name(true)
                                .await
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("failed to send invoice {:?}", e);
                                }
                            }
                        }
                    },
                    Err(e) => {
                        error!("Error in reply: {}", e);
                        bot.send_message(msg.chat.id, e.to_string()).await?;
                    }
                }
            }
        }
        _ => {
            if let Some(id) = q.inline_message_id {
                bot.edit_message_text_inline(id, "Failed to parse inline query")
                    .await?;
            }
        }
    }
    Ok(())
}

/*async fn send_invoice(
    bot: AutoSend<Bot>,
    context: Arc<Context>,
    invoice: Reply,
    need_name: bool,
) -> Result<(), RequestError> {
    match bot
        .send_invoice(
            msg.chat.id,
            invoice.title,
            invoice.description,
            invoice.payload,
            &context.config.payment_provider_token,
            &invoice.currency,
            vec![LabeledPrice {
                label: invoice.currency.to_owned(),
                amount: invoice.amount as i32,
            }],
        )
        .need_name(need_name)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!("failed to send invoice {:?}", e);
        }
    }
    Ok(())
}*/
async fn pre_checkout_handler(
    pre_checkout: PreCheckoutQuery,
    bot: AutoSend<Bot>,
    context: Arc<Context>,
) -> Result<(), RequestError> {
    trace!("pre_checkout_handler::received {:?}", pre_checkout);
    let u = crate::types::User::new(&pre_checkout.from, &context.admins);
    if let Ok(conn) = context.pool.get() {
        let mut lock = context.sign_up_mutex.lock().await;
        *lock = *lock + 1;

        let reply = crate::payments::pre_checkout(&conn, &u, &pre_checkout, &context);
        match reply {
            Ok(_) => {
                bot.answer_pre_checkout_query(pre_checkout.id, true).await?;
            }
            Err(e) => {
                bot.answer_pre_checkout_query(pre_checkout.id, false)
                    .await?;
                bot.send_message(u.id, e.to_string()).await?;
            }
        }
    }
    Ok(())
}

/// Bulk mailing and houskeeping task
async fn perform_bulk_tasks(bot: AutoSend<Bot>, ctx: Arc<Context>) -> Result<bool, RequestError> {
    let mut next_break = tokio::time::Instant::now() + Duration::from_millis(1000);
    loop {
        tokio::time::sleep_until(next_break).await;

        let mut notifications = 0;
        let mut batch_contains_waiting_list_prompt = false;
        let ts = get_unix_time();
        let num_seconds_from_midnight = ts % 86400;

        if num_seconds_from_midnight >= ctx.config.mailing_hours_from.unwrap()
            && num_seconds_from_midnight < ctx.config.mailing_hours_to.unwrap()
        {
            let messages = if let Ok(conn) = ctx.pool.get() {
                match db::get_pending_messages(
                    &conn,
                    ts,
                    ctx.config.limit_bulk_notifications_per_second,
                ) {
                    Ok(messages) => messages,
                    Err(_e) => return Ok(false),
                }
            } else {
                Vec::new()
            };

            for m in messages {
                notifications += m.recipients.len();
                let keyboard: Vec<Vec<InlineKeyboardButton>> =
                    vec![vec![InlineKeyboardButton::callback(
                        "К мероприятию",
                        if m.is_paid {
                            serde_json::to_string(&message_handler::CallbackQuery::PaidEvent {
                                event_id: m.event_id,
                                adults: 0,
                                children: 0,
                                offset: 0,
                            })
                        } else {
                            serde_json::to_string(&message_handler::CallbackQuery::Event {
                                event_id: m.event_id,
                                offset: 0,
                            })
                        }
                        .unwrap(),
                    )]];
                let keyboard = InlineKeyboardMarkup::new(keyboard);
                for u in m.recipients {
                    debug!("Sending notification {} to {} {}", m.message_id, u, &m.text);
                    bot.send_message(UserId(u), &m.text)
                        .parse_mode(ParseMode::Html)
                        .disable_web_page_preview(true)
                        .reply_markup(keyboard.clone())
                        .await?;

                    if let Ok(conn) = ctx.pool.get() {
                        if let Err(e) = db::save_receipt(&conn, m.message_id, u) {
                            error!("Failed to save receipt: {}", e);
                        }
                    }
                    if m.message_type == MessageType::WaitingListPrompt {
                        batch_contains_waiting_list_prompt = true;
                    }
                }
            }
        }

        if ctx.config.cleanup_old_events {
            if let Ok(conn) = ctx.pool.get() {
                // Clean up.
                if db::clear_old_events(
                    &conn,
                    ts - ctx.config.drop_events_after_hours * 60 * 60,
                    ctx.config.automatic_blacklisting,
                    ctx.config.cancel_future_reservations_on_ban,
                    &ctx.admins,
                )
                .is_ok()
                    == false
                {
                    error!("Failed to clear old events at {}", ts);
                }

                // Age black lists.
                if db::clear_black_list(
                    &conn,
                    ts - ctx.config.delete_from_black_list_after_days * 24 * 60 * 60,
                )
                .is_ok()
                    == false
                {
                    error!("Failed to clear black list at {}", ts);
                }
            }
        }

        // Clear failed payments.
        if let Ok(conn) = ctx.pool.get() {
            if db::clear_failed_payments(&conn, ts - 5 * 60).is_ok() == false {
                error!("Failed to clear failed payments at {}", ts);
            }
        }

        next_break = tokio::time::Instant::now()
            + Duration::from_millis(
                if notifications > 0 && batch_contains_waiting_list_prompt == false {
                    1000
                } else {
                    30000
                },
            );
    }
}
