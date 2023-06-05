use crate::get_unix_time;
use crate::payments::{donate, prepare_invoice, show_paid_event};
use crate::reply::*;
use crate::types::{Context, EventState, EventType, ReservationState, User};
use anyhow::anyhow;
use teloxide::{types::InlineKeyboardButton, utils::html};
use url::Url;

use crate::db;
use crate::format;
use db::EventStats;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use serde_compact::compact;

// Testing
use crate::crypto_bindings::{crpt};

/// User dialog handler.
/// Command line processor.
pub fn handle_message(
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
        "/start" => {
            if pars.len() == 2 {
                // Direct link
                if pars[1].starts_with("donate_") {
                    if let Ok(amount) = pars[1][7..].parse::<u64>() {
                        return donate(user, amount, ctx);
                    }
                } else {
                    if let Ok(event_id) = pars[1].parse::<u64>() {
                        return show_event(conn, user, event_id, ctx, None, 0);
                    }
                }
            } else {
                return show_event_list(conn, user.id.0, ctx, 0);
            }
        }
        "/donate" => {
            return donate(user, 500, ctx);
        }

        "/crt" => {
            
        }

        "/help" => {
            return Ok(ReplyMessage::new(format!(
                "Здесь вы можете бронировать места на мероприятия.\n \
                            \n /start - показать список мероприятий \
                            \n /help - эта подсказка \
                            \n <a href=\"{}\">Подробная инструкция</a> \
                            \n /donate - поддержать канал. \
                            \n /crt - TON Testnet Transaction Test",
                ctx.config.help
            ))
            .into());
        }
        _ => {
            // Message from user - try to add as attachment to the last reservation.
            return add_attachment(conn, &user, data, ctx);
        }
    }
    Err(anyhow!("Unknown command"))
}

#[compact]
#[derive(Serialize, Deserialize, Clone)]
pub enum CallbackQuery {
    EventList {
        offset: u64,
    },
    Event {
        event_id: u64,
        offset: u64,
    },
    SignUp {
        event_id: u64,
        is_adult: bool,
        wait: bool,
    },
    Cancel {
        event_id: u64,
        is_adult: bool,
    },
    WontGo {
        event_id: u64,
    },
    ShowWaitingList {
        event_id: u64,
        offset: u64,
    },
    ShowPresenceList {
        event_id: u64,
        offset: u64,
    },
    ConfirmPresence {
        event_id: u64,
        user_id: u64,
        offset: u64,
    },
    PaidEvent {
        event_id: u64,
        adults: u64,
        children: u64,
        offset: u64,
    },
    SendInvoice {
        event_id: u64,
        adults: u64,
        children: u64,
    },

    // admin callbacks
    ChangeEventState {
        event_id: u64,
        state: u64,
    },
    ShowBlackList {
        offset: u64,
    },
    RemoveFromBlackList {
        user_id: u64,
    },
    ConfirmRemoveFromBlackList {
        user_id: u64,
    },
}

/// Callback query processor.
pub fn handle_callback(
    conn: &PooledConnection<SqliteConnectionManager>,
    user: &User,
    data: &str,
    ctx: &Context,
) -> anyhow::Result<Reply> {
    if let Ok(q) = serde_json::from_str::<CallbackQuery>(&data) {
        use CallbackQuery::*;
        match q {
            EventList { offset } => show_event_list(conn, user.id.0, ctx, offset),
            Event { event_id, offset } => show_event(conn, user, event_id, ctx, None, offset),
            SignUp {
                event_id,
                is_adult,
                wait,
            } => {
                match db::sign_up(
                    conn,
                    event_id,
                    user,
                    is_adult as u64,
                    !is_adult as u64,
                    wait as u64,
                    get_unix_time(),
                    0,
                ) {
                    Ok((_, black_listed)) => show_event(
                        conn,
                        user,
                        event_id,
                        ctx,
                        if black_listed {
                            Some(format!("\n\nИзвините, но бронирование невозможно, поскольку ранее Вы не использовали и не отменили бронь. \
                                    Если это ошибка, пожалуйста, свяжитесь с <a href=\"tg://user?id={}\">поддержкой</a> и сообщите код {}. <a href=\"{}\">Инструкция</a>.", ctx.config.support, user.id, ctx.config.help))
                        } else {
                            None
                        },
                        0,
                    ),
                    Err(e) => Err(anyhow!("{}", e)),
                }
            }
            Cancel { event_id, is_adult } => {
                let user_id = user.id.0;
                match db::cancel(conn, event_id, user_id, is_adult as u64) {
                    Ok(_) => {
                        let mut ps = None;
                        if is_too_late_to_cancel(conn, event_id, user, ctx) {
                            if let Ok(s) = db::get_event(conn, event_id, user_id) {
                                if s.adults.my_reservation + s.children.my_reservation == 0
                                    && s.event.adult_ticket_price == 0
                                    && s.event.child_ticket_price == 0
                                {
                                    // Complete cancellation
                                    if db::ban_user(
                                        conn,
                                        user_id,
                                        &user.user_name1,
                                        &user.user_name2,
                                        &format!(
                                            "late cancel {} {}",
                                            format::ts(s.event.ts),
                                            s.event.name
                                        ),
                                        ctx.config.cancel_future_reservations_on_ban,
                                    )
                                    .is_ok()
                                        == false
                                    {
                                        return Err(anyhow!(
                                            "Failed to add user {} to black list",
                                            user.id
                                        ));
                                    }
                                    ps = Some(format!("\n\nВНИМАНИЕ!\nК сожалению, вы отказались от билетов слишком поздно и не сможете больше бронировать бесплатные билеты."));
                                }
                            }
                        }
                        show_event(conn, user, event_id, ctx, ps, 0)
                    }
                    Err(e) => Err(anyhow!("Failed to cancel reservation: {}.", e)),
                }
            }
            WontGo { event_id } => {
                match db::wontgo(conn, event_id, user.id.0) {
                    Ok(_) => {
                        if is_too_late_to_cancel(conn, event_id, user, ctx) {
                            Ok(ReplyMessage::new("К сожалению, вы отказываетесь от билетов слишком поздно и не сможете больше бронировать бесплатные билеты.".to_string()).into())
                        } else {
                            Ok(ReplyMessage::new("Мы сожалеем, что вы не сможете пойти. Увидимся в другой раз. Спасибо!".to_string()).into())
                        }
                    }
                    Err(e) => Err(anyhow!("Failed to add event: {}.", e)),
                }
            }
            ShowWaitingList { event_id, offset } => {
                if ctx.config.public_lists || user.is_admin != false {
                    show_waiting_list(conn, user, event_id, ctx, offset)
                } else {
                    Err(anyhow!("not allowed"))
                }
            }
            ShowPresenceList { event_id, offset } => {
                show_presence_list(conn, event_id, user, ctx, offset)
            }
            ConfirmPresence {
                event_id,
                user_id,
                offset,
            } => {
                let user_has_permissions = if user.is_admin {
                    true
                } else {
                    match db::is_group_leader(conn, event_id, user.id.0) {
                        Ok(res) => res,
                        Err(_) => false,
                    }
                };
                if user_has_permissions {
                    match db::confirm_presence(conn, event_id, user_id) {
                        Ok(_) => show_presence_list(conn, event_id, user, ctx, offset),
                        Err(e) => Err(anyhow!("Failed to confirm presence: {}.", e)),
                    }
                } else {
                    Err(anyhow!("not allowed"))
                }
            }
            PaidEvent {
                event_id,
                adults,
                children,
                offset,
            } => show_paid_event(event_id, adults, children, offset, conn, user, ctx),
            SendInvoice {
                event_id,
                adults,
                children,
            } => prepare_invoice(event_id, adults, children, conn, user, ctx),
            _ => Err(anyhow!("Not allowed.")),
        }
    } else {
        Err(anyhow!("Failed to parse query."))
    }
}

pub fn add_attachment(
    conn: &PooledConnection<SqliteConnectionManager>,
    user: &User,
    data: &str,
    ctx: &Context,
) -> anyhow::Result<Reply> {
    let user_id: u64 = user.id.0;
    match db::get_current_event(conn, user_id) {
        Ok(event_id) if event_id > 0 => {
            match db::add_attachment(conn, event_id, user_id, &html::escape(data)) {
                Ok(_v) => {
                    return show_event(
                        conn,
                        user,
                        event_id,
                        ctx,
                        if data.chars().any(char::is_numeric) {
                            Some("\n\nВНИМАНИЕ!\nВаше примечание содержит цифры. Они никак не влияют на количество забронированных мест. Количество мест можно менять только кнопками \"Записать/Отписать\".".to_string())
                        } else {
                            None
                        },
                        0,
                    )
                }
                _ => return Err(anyhow!("Failed to parse attachment: {}", data)),
            }
        }
        _ => return Err(anyhow!("Failed to find event")),
    }
}

pub fn show_event_list(
    conn: &PooledConnection<SqliteConnectionManager>,
    user_id: u64,
    ctx: &Context,
    offset: u64,
) -> anyhow::Result<Reply> {
    match db::get_events(conn, user_id, offset, ctx.config.event_list_page_size) {
        Ok(events) => {
            Ok(
                // header
                ReplyMessage::new(
                    if offset != 0 || events.len() != 0 {
                        format!("Программа\nвремя / взросл.(детск.) места  / мероприятие\n<a href=\"{}\">инструкция</a> /donate", ctx.config.help)
                    } else {
                        "Нет мероприятий.".to_string()
                    }
                )
                .keyboard(
                    events
                    .iter()
                    .map(|s| {
                        let event_type = s.event.get_type();
                        if event_type == EventType::Announcement {
                            if let Ok(url) = Url::parse(&s.event.link) {
                                vec![InlineKeyboardButton::url(format!("ℹ️ {} {}", format::ts(s.event.ts), s.event.name), url)]
                            } else {
                                vec![]
                            }
                        } else {
                            vec![InlineKeyboardButton::callback(
                                format!(
                                    "{} {} / {} / {}",
                                    if s.adults.my_reservation != 0 || s.children.my_reservation != 0 {
                                        "✅"
                                    } else if s.adults.my_waiting != 0 || s.children.my_waiting != 0 {
                                        "⏳"
                                    } else if event_type != EventType::Paid {
                                        "✨" // todo: find a better emoji for free events
                                    } else {
                                        ""
                                    },
                                    format::ts(s.event.ts),
                                    if s.state == EventState::Open {
                                        if s.event.max_adults == 0 || s.event.max_children == 0 {
                                            (s.event.max_adults - s.adults.reserved + s.event.max_children
                                                - s.children.reserved)
                                                .to_string()
                                        } else {
                                            format!(
                                                "{}({})",
                                                s.event.max_adults - s.adults.reserved,
                                                s.event.max_children - s.children.reserved
                                            )
                                        }
                                    } else {
                                        "-".to_string()
                                    },
                                    s.event.name
                                ),
                                if event_type == EventType::Paid {
                                    serde_json::to_string(&CallbackQuery::PaidEvent {
                                        event_id: s.event.id,
                                        adults: 0,
                                        children: 0,
                                        offset: 0,
                                    })
                                } else {
                                    serde_json::to_string(&CallbackQuery::Event {
                                        event_id: s.event.id,
                                        offset: 0,
                                    })
                                }
                                .unwrap(),
                            )]
                        }
                    })
                    .collect()
                )
                // pagination
                .pagination(
                    &CallbackQuery::EventList {offset: offset.saturating_sub(1)},
                    &CallbackQuery::EventList {offset: offset + 1},
                    events.len() as u64,
                    ctx.config.event_list_page_size,
                    offset,
                )?
                .into()
            )
        }
        Err(e) => Err(anyhow!("Failed to query events: {}", e)),
    }
}

pub fn show_event(
    conn: &PooledConnection<SqliteConnectionManager>,
    user: &User,
    event_id: u64,
    ctx: &Context,
    ps: Option<String>,
    offset: u64,
) -> anyhow::Result<Reply> {
    match db::get_event(conn, event_id, user.id.0) {
        Ok(s) => {
            let free_adults = s.event.max_adults as i64 - s.adults.reserved as i64;
            let free_children = s.event.max_children as i64 - s.children.reserved as i64;
            let no_age_distinction = s.event.max_adults == 0 || s.event.max_children == 0;
            let is_admin = ctx.config.admins.contains(&user.id.0);
            let (participants, participants_len) = if ctx.config.public_lists || is_admin {
                let participants = db::get_participants(
                    conn,
                    event_id,
                    0,
                    offset,
                    ctx.config.event_page_size,
                    ReservationState::Free,
                )?;
                let len = participants.len();
                (Some(participants), len as u64)
            } else {
                (None, 0)
            };

            Ok(
                // header
                ReplyMessage::new(
                    format::header(
                        &s,
                        free_adults,
                        free_children,
                        is_admin,
                        no_age_distinction,
                    )
                )
                // participants
                .text(participants.and_then(|participants| {
                    Some(format::participants(
                        &s,
                        &participants,
                        is_admin,
                        no_age_distinction,
                    ))
                }))
                // messages
                .text(format::messages(conn, &s, event_id, is_admin))
                // attachment
                .text({
                    if is_admin
                        || s.adults.my_reservation > 0
                        || s.adults.my_waiting > 0
                        || s.children.my_reservation > 0
                        || s.children.my_waiting > 0
                    {
                        let mut text = "".to_string();
                        if ctx.config.public_lists == false {
                            match db::get_attachment(conn, event_id, user.id.0) {
                                Ok(v) => {
                                    if let Some(attachment) = v {
                                        text.push_str(&format!("\nПримечание: {}.", attachment));
                                    }
                                }
                                Err(e) => error!("Failed to get attachment: {}", e),
                            }
                        }
                        if is_admin == false {
                            text.push_str("\nКоличество мест можно менять кнопками \"Записаться/Отписаться\". Примечание к брони можно добавить, послав сообщение боту.\n");
                        }
                        if s.adults.my_reservation + s.children.my_reservation > 0 {
                            text.push_str(&format!(
                                "\n<b>У вас забронировано: {}</b>",
                                s.adults.my_reservation + s.children.my_reservation
                            ));
                        }
                        if s.adults.my_waiting + s.children.my_waiting > 0 {
                            text.push_str(&format!(
                                "\n<b>У вас в списке ожидания: {}</b>",
                                s.adults.my_waiting + s.children.my_waiting
                            ));
                        }
                        Some(text)
                    } else {
                        None
                    }
                })
                // controls
                .keyboard(get_signup_controls(
                    &s,
                    free_adults,
                    free_children,
                    no_age_distinction,
                    is_admin,
                    user.id.0,
                    conn,
                )?)
                // pagination
                .pagination(
                    &CallbackQuery::Event {
                        event_id,
                        offset: offset.saturating_sub(1),
                    },
                    &CallbackQuery::Event {
                        event_id,
                        offset: offset + 1,
                    },
                    participants_len,
                    ctx.config.event_page_size,
                    offset,
                )?
                .text(ps)
                .into()
            )
        }
        Err(e) => Err(anyhow!("Failed to fetch event: {}", e)),
    }
}

fn get_signup_controls(
    s: &EventStats,
    free_adults: i64,
    free_children: i64,
    no_age_distinction: bool,
    is_admin: bool,
    user_id: u64,
    conn: &PooledConnection<SqliteConnectionManager>,
) -> anyhow::Result<Vec<Vec<InlineKeyboardButton>>> {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let mut row: Vec<InlineKeyboardButton> = Vec::new();
    if s.state == EventState::Open && s.adults.my_reservation < s.event.max_adults_per_reservation {
        if free_adults > 0 {
            row.push(InlineKeyboardButton::callback(
                if no_age_distinction {
                    "Записаться +1"
                } else {
                    "Записать взрослого +1"
                },
                &serde_json::to_string(&CallbackQuery::SignUp {
                    event_id: s.event.id,
                    is_adult: true,
                    wait: false,
                })?,
            ));
        } else if s.adults.my_reservation + s.adults.my_waiting < s.event.max_adults_per_reservation
        {
            row.push(InlineKeyboardButton::callback(
                if no_age_distinction {
                    "В лист ожидания +1"
                } else {
                    "В лист ожидания взрослого +1"
                },
                &serde_json::to_string(&CallbackQuery::SignUp {
                    event_id: s.event.id,
                    is_adult: true,
                    wait: true,
                })?,
            ));
        }
    }
    if s.adults.my_reservation > 0 || s.adults.my_waiting > 0 {
        row.push(InlineKeyboardButton::callback(
            if no_age_distinction {
                "Отписаться -1"
            } else {
                "Отписать взрослого -1"
            },
            &serde_json::to_string(&CallbackQuery::Cancel {
                event_id: s.event.id,
                is_adult: true,
            })?,
        ));
    }
    keyboard.push(row);
    row = Vec::new();
    if s.state == EventState::Open
        && s.children.my_reservation < s.event.max_children_per_reservation
    {
        if free_children > 0 {
            row.push(InlineKeyboardButton::callback(
                if no_age_distinction {
                    "Записаться +1"
                } else {
                    "Записать ребёнка +1"
                },
                &serde_json::to_string(&CallbackQuery::SignUp {
                    event_id: s.event.id,
                    is_adult: false,
                    wait: false,
                })?,
            ));
        } else if s.children.my_reservation + s.children.my_waiting
            < s.event.max_children_per_reservation
        {
            row.push(InlineKeyboardButton::callback(
                if no_age_distinction {
                    "В лист ожидания +1"
                } else {
                    "В лист ожидания ребёнка +1"
                },
                &serde_json::to_string(&CallbackQuery::SignUp {
                    event_id: s.event.id,
                    is_adult: false,
                    wait: true,
                })?,
            ));
        }
    }
    if s.children.my_reservation > 0 || s.children.my_waiting > 0 {
        row.push(InlineKeyboardButton::callback(
            if no_age_distinction {
                "Отписаться -1"
            } else {
                "Отписать ребёнка -1"
            },
            &serde_json::to_string(&CallbackQuery::Cancel {
                event_id: s.event.id,
                is_adult: false,
            })?,
        ));
    }
    keyboard.push(row);

    row = Vec::new();
    row.push(InlineKeyboardButton::callback(
        "Список мероприятий",
        serde_json::to_string(&CallbackQuery::EventList { offset: 0 })?,
    ));

    let event_id = s.event.id;
    if s.adults.reserved > 0 || s.children.reserved > 0 {
        row.push(InlineKeyboardButton::callback(
            "Список ожидания",
            &serde_json::to_string(&CallbackQuery::ShowWaitingList {
                event_id,
                offset: 0,
            })?,
        ));
    }

    if is_admin {
        if s.adults.reserved > 0 || s.children.reserved > 0 {
            row.push(InlineKeyboardButton::callback(
                "Присутствие",
                &serde_json::to_string(&CallbackQuery::ShowPresenceList {
                    event_id,
                    offset: 0,
                })?,
            ));
        }
        if s.state == EventState::Open {
            row.push(InlineKeyboardButton::callback(
                "Остановить запись",
                serde_json::to_string(&CallbackQuery::ChangeEventState { event_id, state: 1 })?,
            ));
        } else {
            row.push(InlineKeyboardButton::callback(
                "Разрешить запись",
                serde_json::to_string(&CallbackQuery::ChangeEventState { event_id, state: 0 })?,
            ));
        }
    } else {
        if s.adults.reserved > 0 || s.children.reserved > 0 {
            if let Ok(check) = db::is_group_leader(conn, event_id, user_id) {
                if check {
                    row.push(InlineKeyboardButton::callback(
                        "Присутствие",
                        &serde_json::to_string(&CallbackQuery::ShowPresenceList {
                            event_id,
                            offset: 0,
                        })?,
                    ));
                }
            }
        }
    }
    keyboard.push(row);
    Ok(keyboard)
}

fn show_waiting_list(
    conn: &PooledConnection<SqliteConnectionManager>,
    user: &User,
    event_id: u64,
    ctx: &Context,
    offset: u64,
) -> anyhow::Result<Reply> {
    let mut list = "".to_string();
    let no_age_distinction;
    let is_admin = ctx.config.admins.contains(&user.id.0);
    match db::get_event(conn, event_id, user.id.0) {
        Ok(s) => {
            no_age_distinction = s.event.max_adults == 0 || s.event.max_children == 0;
            list.push_str(&format!(
                "\n \n{}\nНачало: {}\n",
                format::event_title(&s.event),
                format::ts(s.event.ts)
            ));
        }
        Err(e) => {
            return Err(anyhow!("Failed to find event: {}", e));
        }
    }
    match db::get_participants(
        conn,
        event_id,
        1,
        offset,
        ctx.config.event_page_size,
        ReservationState::Free,
    ) {
        Ok(participants) => {
            Ok(ReplyMessage::new(if participants.len() == 0 {
                "Пустой список ожидания.".to_string()
            } else {
                "Список ожидания:\n".to_string()
                    + &participants
                        .iter()
                        .map(|p| {
                            let mut entry = "".to_string();
                            let id = if is_admin {
                                p.user_id.to_string()
                            } else {
                                "".to_string()
                            };
                            if p.user_name2.len() > 0 {
                                entry.push_str(&format!(
                                    "\n{} <a href=\"https://t.me/{}\">{} ({})</a>",
                                    id, p.user_name2, p.user_name1, p.user_name2
                                ));
                            } else {
                                entry.push_str(&format!(
                                    "\n{} <a href=\"tg://user?id={}\">{}</a>",
                                    id, p.user_id, p.user_name1
                                ));
                            }
                            if no_age_distinction {
                                entry.push_str(&format!(" {}", p.adults + p.children));
                            } else {
                                entry.push_str(&format!(" {}({})", p.adults, p.children));
                            }
                            if let Some(a) = &p.attachment {
                                entry.push_str(&format!(" {}", a));
                            }
                            entry
                        })
                        .collect::<String>()
            })
            // controls
            .keyboard(vec![vec![InlineKeyboardButton::callback(
                "Назад",
                &serde_json::to_string(&CallbackQuery::Event {
                    event_id,
                    offset: 0,
                })?,
            )]])
            // pagination
            .pagination(
                &CallbackQuery::ShowWaitingList {
                    event_id,
                    offset: offset.saturating_sub(1),
                },
                &CallbackQuery::ShowWaitingList {
                    event_id,
                    offset: offset + 1,
                },
                participants.len() as u64,
                ctx.config.event_page_size,
                offset,
            )?
            .into())
        }
        Err(e) => Err(anyhow!("Failed to get participants: {}", e)),
    }
}

fn is_too_late_to_cancel(
    conn: &PooledConnection<SqliteConnectionManager>,
    event_id: u64,
    user: &User,
    ctx: &Context,
) -> bool {
    if let Ok(s) = db::get_event(conn, event_id, user.id.0) {
        if s.event.ts - get_unix_time() < ctx.config.too_late_to_cancel_hours * 60 * 60 {
            return true;
        }
    }
    false
}

fn show_presence_list(
    conn: &PooledConnection<SqliteConnectionManager>,
    event_id: u64,
    user: &User,
    ctx: &Context,
    offset: u64,
) -> anyhow::Result<Reply> {
    let mut header = "".to_string();
    match db::get_event(conn, event_id, user.id.0) {
        Ok(s) => {
            header.push_str(&format!(
                "\n \n{}\nНачало: {}\n",
                format::event_title(&s.event),
                format::ts(s.event.ts)
            ));
        }
        Err(e) => {
            return Err(anyhow!("Failed to find event: {}", e));
        }
    }
    match db::get_presence_list(conn, event_id, offset, ctx.config.presence_page_size) {
        Ok(participants) => {
            Ok(
                // header
                ReplyMessage::new(if participants.len() == 0 {
                    "Пустой список ожидания."
                } else {
                    "Пожалуйста, выберите присутствующих:\n"
                })
                .keyboard(
                    participants
                        .iter()
                        .map(|p| {
                            vec![{
                                let mut text;
                                if p.user_name2.len() > 0 {
                                    text = format!(
                                        "{} ({}) {}",
                                        p.user_name1, p.user_name2, p.reserved
                                    );
                                } else {
                                    text = format!("{} {}", p.user_name1, p.reserved);
                                }
                                if let Some(a) = &p.attachment {
                                    text.push_str(&format!(" - {}", a));
                                }

                                InlineKeyboardButton::callback(
                                    text,
                                    &serde_json::to_string(&CallbackQuery::ConfirmPresence {
                                        event_id,
                                        user_id: p.user_id,
                                        offset: offset,
                                    })
                                    .unwrap(),
                                )
                            }]
                        })
                        .collect(),
                )
                // controls
                .keyboard(vec![vec![InlineKeyboardButton::callback(
                    "Назад",
                    &serde_json::to_string(&CallbackQuery::Event {
                        event_id,
                        offset: 0,
                    })?,
                )]])
                // pagination
                .pagination(
                    &CallbackQuery::ShowPresenceList {
                        event_id,
                        offset: offset.saturating_sub(1),
                    },
                    &CallbackQuery::ShowPresenceList {
                        event_id,
                        offset: offset + 1,
                    },
                    participants.len() as u64,
                    ctx.config.event_page_size,
                    offset,
                )?
                .into(),
            )
        }
        Err(e) => Err(anyhow!("Failed to get precense list: {}", e)),
    }
}
