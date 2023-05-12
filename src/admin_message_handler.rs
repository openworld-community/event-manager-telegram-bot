use std::env;
use crate::db;
use crate::format;
use crate::message_handler;
use crate::message_handler::CallbackQuery;
use crate::reply::*;
use crate::types::{Configuration, Context, Event, MessageType, User};
use anyhow::anyhow;
use chrono::DateTime;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use teloxide::{
    types::{InlineKeyboardButton, ParseMode},
    utils::markdown,
};

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
struct NewEvent {
    id: Option<u64>,
    name: String,
    link: String,
    start: String,
    remind: String,
    max_adults: u64,
    max_children: u64,
    max_adults_per_reservation: u64,
    max_children_per_reservation: u64,
    adult_ticket_price: Option<f64>,
    child_ticket_price: Option<f64>,
    currency: String,
}

/// Command line processor.
pub fn handle_message(
    conn: &PooledConnection<SqliteConnectionManager>,
    user: &User,
    data: &str,
    ctx: &Context,
) -> anyhow::Result<Reply> {
    let pars: Vec<&str> = data.splitn(4, ' ').collect();
    if pars.len() == 0 {
        return Err(anyhow!("Unknown command"));
    }
    match pars[0] {
        "/send" if pars.len() == 4 => {
            // Broadcast message to a group?
            // /send confirmed <event> text
            // /send waiting <event> text
            let waiting_list = match pars[1] {
                "confirmed" => 0,
                "waiting" => 1,
                _ => 2,
            };
            if waiting_list < 2 {
                if let Ok(event_id) = pars[2].parse::<u64>() {
                    match db::get_event(conn, event_id, user.id.0) {
                        Ok(s) => {
                            let text = format!(
                                    "<a href=\"tg://user?id={}\">{}</a>:\nСообщение по мероприятию {} (Начало: {})\n{}",
                                    user.id.0,
                                    user.user_name1,
                                    format::event_title(&s.event),
                                    format::ts(s.event.ts),
                                    pars[3].to_string()
                                );

                            if db::enqueue_message(
                                conn,
                                event_id,
                                &user.user_name1,
                                waiting_list,
                                MessageType::Direct,
                                &text,
                                crate::util::get_unix_time(),
                            )
                            .is_ok()
                            {
                                return Ok(ReplyMessage::new(format!(
                                    "The following message has been scheduled for sending:\n{}",
                                    text
                                ))
                                .into());
                            } else {
                                return Ok(ReplyMessage::new("Failed to send message.").into());
                            }
                        }
                        Err(e) => {
                            return Err(anyhow!("Failed to find event: {}", e));
                        }
                    }
                }
            }
        }
        "/ban" if pars.len() == 2 => {
            if let Ok(user_id) = pars[1].parse::<u64>() {
                if db::add_to_black_list(
                    conn,
                    user_id,
                    ctx.config.cancel_future_reservations_on_ban,
                )
                .is_ok()
                    == false
                {
                    error!("Failed to add user {} to black list", user_id);
                }
                return show_black_list(conn, &ctx.config, 0);
            }
        }
        "/remove_from_black_list" if pars.len() == 2 => {
            if let Ok(user_id) = pars[1].parse::<u64>() {
                if db::remove_from_black_list(conn, user_id).is_ok() == false {
                    error!("Failed to remove user {} from black list", user_id);
                }
                return show_black_list(conn, &ctx.config, 0);
            }
        }
        "/delete_event" if pars.len() == 2 => {
            if let Ok(event_id) = pars[1].parse::<u64>() {
                match db::delete_event(conn, event_id, ctx.config.automatic_blacklisting,
                    ctx.config.cancel_future_reservations_on_ban,
                    &ctx.admins) {
                    Ok(_) => {
                        return Ok(ReplyMessage::new("Deleted").into());
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to delete event: {}.", e));
                    }
                }
            }
        }
        "/delete_link" if pars.len() == 2 => {
            match db::delete_link(conn, pars[1]) {
                Ok(_) => {
                    return Ok(ReplyMessage::new("Deleted").into());
                }
                Err(e) => {
                    return Err(anyhow!("Failed to delete link: {}.", e));
                }
            }
        }
        "/delete_reservation" if pars.len() == 3 => {
            if let (Ok(event_id), Ok(user_id)) = (pars[1].parse::<u64>(), pars[2].parse::<u64>()) {
                match db::delete_reservation(conn, event_id, user_id) {
                    Ok(_) => {
                        return Ok(ReplyMessage::new("Reservation deleted.").into());
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to delete reservation: {}.", e));
                    }
                };
            }
        }
        "/set_group_leader" if pars.len() == 3 => {
            if let (Ok(event_id), Ok(user_id)) = (pars[1].parse::<u64>(), pars[2].parse::<u64>()) {
                match db::set_group_leader(conn, event_id, user_id) {
                    Ok(_) => {
                        return Ok(ReplyMessage::new("Group leader set.").into());
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to set group leader: {}.", e));
                    }
                };
            }
        }
        "/show_black_list" => {
            return show_black_list(conn, &ctx.config, 0);
        }
        "/set_event_limits" if pars.len() == 4 => {
            if let (Ok(event_id), Ok(max_adults), Ok(max_children)) = (
                pars[1].parse::<u64>(),
                pars[2].parse::<u64>(),
                pars[3].parse::<u64>(),
            ) {
                match db::set_event_limits(conn, event_id, max_adults, max_children) {
                    Ok(_) => {
                        return Ok(ReplyMessage::new("Event limits updated.").into());
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to set event limits: {}.", e));
                    }
                };
            }
        }
        "/help" => {
            return Ok(ReplyMessage::new(markdown::escape(
                        "Добавить мероприятие: \
                        \n { \"name\":\"тест\", \"link\":\"https://t.me/storiesvienna/21\", \"start\":\"2022-05-29 15:00 +02:00\", \"remind\":\"2022-05-28 15:00 +02:00\", \"max_adults\":15, \"max_children\":15, \"max_adults_per_reservation\":15, \"max_children_per_reservation\":15, \"currency\":\"EUR\" }\
                        \n\n Отредактировать: добавьте \"id\":<event> в команду выше \
                        \n\n Цены билетов: добавьте \"adult_ticket_price\":200, \"child_ticket_price\":100 в выбранной валюте в команду выше \
                        \n \nПослать сообщение: \
                        \n /send confirmed <event> текст \
                        \n /send waiting <event> текст \
                        \n \nЧёрный список: \
                        \n /ban <user> \
                        \n /show_black_list \
                        \n \
                        \n /delete_event <event> \
                        \n /delete_link <url> \
                        \n /delete_reservation <event> <user> \
                        \n /set_group_leader <event> <user> \
                        \n /set_event_limits <event> <max_adults> <max_children> \
                        ")).parse_mode(ParseMode::MarkdownV2).into());
        }
        _ => {
            if let Some(ch) = data.chars().next() {
                if ch == '{' {
                    return add_event(conn, data);
                }
            }
            return crate::message_handler::handle_message(conn, user, data, ctx);
        }
    }
    Err(anyhow!("Failed to parse command"))
}

/// Callback query processor.
pub fn handle_callback(
    conn: &PooledConnection<SqliteConnectionManager>,
    user: &User,
    data: &str,
    ctx: &Context,
) -> anyhow::Result<Reply> {
    match serde_json::from_str::<CallbackQuery>(&data) {
        Ok(q) => {
            use CallbackQuery::*;
            match q {
                ChangeEventState { event_id, state } => {
                    match db::change_event_state(conn, event_id, state) {
                        Ok(_) => message_handler::show_event(conn, user, event_id, ctx, None, 0),
                        Err(e) => Err(anyhow!("Failed to close event: {}.", e)),
                    }
                }
                ShowBlackList { offset } => show_black_list(conn, &ctx.config, offset),
                RemoveFromBlackList { user_id } => {
                    if db::remove_from_black_list(conn, user_id).is_ok() == false {
                        error!("Failed to remove user {} from black list", user_id);
                    }
                    show_black_list(conn, &ctx.config, 0)
                }
                ConfirmRemoveFromBlackList { user_id } => {
                    if let Ok(reason) = db::get_ban_reason(conn, user_id) {
                        let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![vec![
                            InlineKeyboardButton::callback(
                                "да",
                                serde_json::to_string(&RemoveFromBlackList { user_id })?,
                            ),
                            InlineKeyboardButton::callback(
                                "нет",
                                serde_json::to_string(&ShowBlackList { offset: 0 })?,
                            ),
                        ]];
                        Ok(ReplyMessage::new(format!("Причина бана: {reason}\nУдалить пользавателя <a href=\"tg://user?id={0}\">{0}</a> из чёрного списка?", user_id)).keyboard(keyboard).into())
                    } else {
                        Err(anyhow!("Failed to find ban reason"))
                    }
                }
                _ => {
                    // Try user message.
                    message_handler::handle_callback(conn, user, data, ctx)
                }
            }
        }
        Err(e) => Err(anyhow!("Failed to parse callback: {}.", e)),
    }
}

fn add_event(
    conn: &PooledConnection<SqliteConnectionManager>,
    data: &str,
) -> anyhow::Result<Reply> {
    match serde_json::from_str::<NewEvent>(&data) {
        Ok(v) => {
            match (
                DateTime::parse_from_str(&v.start, "%Y-%m-%d %H:%M  %z"),
                DateTime::parse_from_str(&v.remind, "%Y-%m-%d %H:%M  %z"),
            ) {
                (Ok(ts), Ok(remind)) => {
                    let event = Event {
                        id: v.id.unwrap_or(0),
                        name: v.name,
                        link: v.link,
                        max_adults: v.max_adults,
                        max_children: v.max_children,
                        max_adults_per_reservation: v.max_adults_per_reservation,
                        max_children_per_reservation: v.max_children_per_reservation,
                        ts: ts.timestamp() as u64,
                        remind: remind.timestamp() as u64,
                        adult_ticket_price: (v.adult_ticket_price.unwrap_or(0.00f64) * 100.0) as u64,
                        child_ticket_price: (v.child_ticket_price.unwrap_or(0.00f64) * 100.0) as u64,
                        currency: v.currency,
                    };

                    if event.adult_ticket_price != 0 && event.max_adults == 0
                        || event.child_ticket_price != 0 && event.max_children == 0
                    {
                        return Err(anyhow!("Wrong event format"));
                    }
                    match crate::db::add_event(conn, event) {
                        Ok(id) => {
                            return Ok(ReplyMessage::new(if id > 0 {
                                let bot_name = env::var("BOT_NAME").unwrap();
                                format!("Direct event link: https://t.me/{}?start={}", bot_name, id)
                            } else {
                                format!("Failed to add event.")
                            }).into());
                        }
                        Err(e) => {
                            return Err(anyhow!("Failed to add event: {}.", e));
                        }
                    }
                }
                _ => {
                    return Err(anyhow!("Failed to parse date"));
                }
            }
        }
        Err(e) => {
            return Err(anyhow!("Failed to parse json: {}", e));
        }
    }
}

fn show_black_list(
    conn: &PooledConnection<SqliteConnectionManager>,
    config: &Configuration,
    offset: u64,
) -> anyhow::Result<Reply> {
    match db::get_black_list(conn, offset, config.presence_page_size) {
        Ok(participants) => {
            Ok(
                // header
                ReplyMessage::new(if participants.len() != 0 || offset > 0 {
                    "Чёрный список. Нажмите кнопку чтобы удалить из списка."
                } else {
                    "Чёрный список пуст."
                })
                // list
                .keyboard(
                    participants
                        .iter()
                        .map(|u| {
                            vec![InlineKeyboardButton::callback(
                                if u.user_name2.len() > 0 {
                                    format!("{} ({}) {}", u.user_name1, u.user_name2, u.id)
                                } else {
                                    format!("{} {}", u.user_name1, u.id)
                                },
                                serde_json::to_string(&CallbackQuery::ConfirmRemoveFromBlackList {
                                    user_id: u.id.0,
                                })
                                .unwrap(),
                            )]
                        })
                        .collect(),
                )
                // pagination
                .pagination(
                    &CallbackQuery::ShowBlackList {
                        offset: offset.saturating_sub(1),
                    },
                    &CallbackQuery::ShowBlackList { offset: offset + 1 },
                    participants.len() as u64,
                    config.presence_page_size,
                    offset,
                )?
                .into(),
            )
        }
        Err(e) => Err(anyhow!("Failed to get black list: {}", e)),
    }
}
