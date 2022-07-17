use crate::db;
use crate::message_handler;
use crate::types::{Configuration, Context, Event, MessageType, Reply, User};
use crate::util::{format_event_title, format_ts};
use anyhow::anyhow;
use chrono::DateTime;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use teloxide::{
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
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
                                    format_event_title(&s.event),
                                    format_ts(s.event.ts),
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
                                return Ok(Reply::new(format!(
                                    "The following message has been scheduled for sending:\n{}",
                                    text
                                )));
                            } else {
                                return Ok(Reply::new(format!("Failed to send message.")));
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
                if ctx.config.automatic_blacklisting {
                    if let Err(e) = db::blacklist_absent_participants(
                        conn,
                        event_id,
                        &ctx.admins,
                        ctx.config.cancel_future_reservations_on_ban,
                    ) {
                        return Err(anyhow!("Failed to blacklist absent participants: {}.", e));
                    }
                }
                match db::delete_event(conn, event_id) {
                    Ok(_) => {
                        return Ok(Reply::new("Deleted".to_string()));
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to delete event: {}.", e));
                    }
                }
            }
        }
        "/delete_reservation" if pars.len() == 3 => {
            if let (Ok(event_id), Ok(user_id)) = (pars[1].parse::<u64>(), pars[2].parse::<u64>()) {
                match db::delete_reservation(conn, event_id, user_id) {
                    Ok(_) => {
                        return Ok(Reply::new(format!("Reservation deleted.")));
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
                        return Ok(Reply::new(format!("Group leader set.")));
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
                        return Ok(Reply::new(format!("Event limits updated.")));
                    }
                    Err(e) => {
                        return Err(anyhow!("Failed to set event limits: {}.", e));
                    }
                };
            }
        }
        "/help" => {
            return Ok(Reply::new(markdown::escape(
                        "Добавить мероприятие: \
                        \n { \"name\":\"WIENXTRA CHILDREN'S ACTIVITIES for children up to 13 y.o.\", \"link\":\"https://t.me/storiesvienna/21\", \"start\":\"2022-05-29 15:00 +02:00\", \"remind\":\"2022-05-28 15:00 +02:00\", \"max_adults\":15, \"max_children\":15, \"max_adults_per_reservation\":15, \"max_children_per_reservation\":15 }\
                        \n\n Отредактировать: добавьте \"id\":<event> в команду выше \
                        \n \nПослать сообщение: \
                        \n /send confirmed <event> текст \
                        \n /send waiting <event> текст \
                        \n \nЧёрный список: \
                        \n /ban <user> \
                        \n /show_black_list \
                        \n \
                        \n /delete_event <event> \
                        \n /delete_reservation <event> <user> \
                        \n /set_group_leader <event> <user> \
                        \n /set_event_limits <event> <max_adults> <max_children> \
                        ")).parse_mode(ParseMode::MarkdownV2));
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
    let pars: Vec<&str> = data.splitn(3, ' ').collect();
    if pars.len() == 0 {
        return Err(anyhow!("Unknown command"));
    }
    match pars[0] {
        "change_event_state" if pars.len() == 3 => {
            match (pars[1].parse::<u64>(), pars[2].parse::<u64>()) {
                (Ok(event_id), Ok(state)) => match db::change_event_state(conn, event_id, state) {
                    Ok(_) => message_handler::show_event(conn, user, event_id, ctx, None, 0),
                    Err(e) => Err(anyhow!("Failed to close event: {}.", e)),
                },
                _ => Err(anyhow!("Failed to parse command: {}", data)),
            }
        }
        "confirm_remove_from_black_list" if pars.len() == 2 => {
            if let Ok(user_id) = pars[1].parse::<u64>() {
                if let Ok(reason) = db::get_ban_reason(conn, user_id) {
                    let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![vec![
                        InlineKeyboardButton::callback(
                            "да",
                            format!("remove_from_black_list {}", user_id),
                        ),
                        InlineKeyboardButton::callback("нет", "show_black_list 0"),
                    ]];
                    Ok(Reply::new(format!("Причина бана: {reason}\nУдалить пользавателя <a href=\"tg://user?id={0}\">{0}</a> из чёрного списка?", user_id)).reply_markup(InlineKeyboardMarkup::new(keyboard)))
                } else {
                    Err(anyhow!("Failed to find ban reason"))
                }
            } else {
                Err(anyhow!("Failed to get user id"))
            }
        }
        "remove_from_black_list" if pars.len() == 2 => {
            if let Ok(user_id) = pars[1].parse::<u64>() {
                if db::remove_from_black_list(conn, user_id).is_ok() == false {
                    error!("Failed to remove user {} from black list", user_id);
                }
                show_black_list(conn, &ctx.config, 0)
            } else {
                Err(anyhow!("Failed to get user id"))
            }
        }
        "show_black_list" if pars.len() == 2 => {
            if let Ok(offset) = pars[1].parse::<u64>() {
                show_black_list(conn, &ctx.config, offset)
            } else {
                Err(anyhow!("Failed to get black list offset"))
            }
        }
        _ => message_handler::handle_callback(conn, user, data, ctx),
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
                    match crate::db::add_event(
                        conn,
                        Event {
                            id: v.id.unwrap_or(0),
                            name: v.name,
                            link: v.link,
                            max_adults: v.max_adults,
                            max_children: v.max_children,
                            max_adults_per_reservation: v.max_adults_per_reservation,
                            max_children_per_reservation: v.max_children_per_reservation,
                            ts: ts.timestamp() as u64,
                            remind: remind.timestamp() as u64,
                        },
                    ) {
                        Ok(id) => {
                            return Ok(Reply::new(if id > 0 {
                                format!("Direct event link: https://t.me/sign_up_for_event_bot?start={}", id)
                            } else {
                                format!("Failed to add event.")
                            }));
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
            let mut keyboard: Vec<Vec<InlineKeyboardButton>> = participants
                .iter()
                .map(|u| {
                    vec![InlineKeyboardButton::callback(
                        if u.user_name2.len() > 0 {
                            format!("{} ({}) {}", u.user_name1, u.user_name2, u.id)
                        } else {
                            format!("{} {}", u.user_name1, u.id)
                        },
                        format!("confirm_remove_from_black_list {}", u.id),
                    )]
                })
                .collect();

            crate::message_handler::add_pagination(
                &mut keyboard,
                "show_black_list",
                participants.len() as u64,
                config.presence_page_size,
                offset,
            );

            let header = if participants.len() != 0 || offset > 0 {
                "Чёрный список. Нажмите кнопку чтобы удалить из списка."
            } else {
                "Чёрный список пуст."
            };
            Ok(Reply::new(header.to_string()).reply_markup(InlineKeyboardMarkup::new(keyboard)))
        }
        Err(e) => Err(anyhow!("Failed to get black list: {}", e)),
    }
}
