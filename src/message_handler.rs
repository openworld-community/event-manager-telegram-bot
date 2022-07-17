use crate::types::{Context, EventState, Reply, User};
use crate::util::format_event_title;
use crate::{format_ts, get_unix_time};
use anyhow::anyhow;
use teloxide::{
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    utils::html,
};

use crate::db;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

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
                if let Ok(event_id) = pars[1].parse::<u64>() {
                    return show_event(conn, user, event_id, ctx, None, 0);
                }
            } else {
                return show_event_list(conn, user.id.0, ctx, 0);
            }
        }
        "/help" => {
            return Ok(Reply::new(format!(
                "Здесь вы можете бронировать места на мероприятия.\n \
                            \n /start - показать список мероприятий \
                            \n /help - эта подсказка \
                            \n <a href=\"{}\">Подробная инструкция</a>.",
                ctx.config.help
            )));
        }
        _ => {
            // Message from user - try to add as attachment to the last reservation.
            return add_attachment(conn, &user, data, ctx);
        }
    }
    Err(anyhow!("Unknown command"))
}

/// Callback query processor.
pub fn handle_callback(
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
        "event_list" if pars.len() == 2 => match pars[1].parse::<u64>() {
            Ok(offset) => show_event_list(conn, user.id.0, ctx, offset),
            _ => Err(anyhow!("Failed to parse query: {}", data)),
        },
        "event" if pars.len() == 3 => match (pars[1].parse::<u64>(), pars[2].parse::<u64>()) {
            (Ok(event_id), Ok(offset)) => show_event(conn, user, event_id, ctx, None, offset),
            _ => Err(anyhow!("Failed to parse query: {}", data)),
        },
        "sign_up" if pars.len() == 4 => match pars[1].parse::<u64>() {
            Ok(event_id) => {
                let is_adult = pars[2] == "adult";
                let wait = pars[3] == "wait";
                match db::sign_up(
                    conn,
                    event_id,
                    user,
                    is_adult as u64,
                    !is_adult as u64,
                    wait as u64,
                    get_unix_time(),
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
            Err(e) => Err(anyhow!("Failed sign up: {}", e)),
        },
        "cancel" if pars.len() == 3 => {
            if let Ok(event_id) = pars[1].parse::<u64>() {
                let is_adult = pars[2] == "adult";
                let user_id = user.id.0;
                match db::cancel(conn, event_id, user_id, is_adult as u64) {
                    Ok(_) => {
                        let mut ps = None;
                        if is_too_late_to_cancel(conn, event_id, user, ctx) {
                            if let Ok(s) = db::get_event(conn, event_id, user_id) {
                                if s.adults.my_reservation + s.children.my_reservation == 0 {
                                    // Complete cancellation
                                    if db::ban_user(
                                        conn,
                                        user_id,
                                        &user.user_name1,
                                        &user.user_name2,
                                        &format!(
                                            "late cancel {} {}",
                                            format_ts(s.event.ts),
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
            } else {
                Err(anyhow!("Failed to get event id."))
            }
        }
        "wontgo" if pars.len() == 2 => {
            match pars[1].parse::<u64>() {
                Ok(event_id) => match db::wontgo(conn, event_id, user.id.0) {
                    Ok(_) => {
                        if is_too_late_to_cancel(conn, event_id, user, ctx) {
                            Ok(Reply::new("К сожалению, вы отказываетесь от билетов слишком поздно и не сможете больше бронировать бесплатные билеты.".to_string()))
                        } else {
                            Ok(Reply::new("Мы сожалеем, что вы не сможете пойти. Увидимся в другой раз. Спасибо!".to_string()))
                        }
                    }
                    Err(e) => Err(anyhow!("Failed to add event: {}.", e)),
                },
                Err(e) => Err(anyhow!("Failed to cancel: {}", e)),
            }
        }
        "change_event_state" if pars.len() == 3 => {
            match (pars[1].parse::<u64>(), pars[2].parse::<u64>()) {
                (Ok(event_id), Ok(state)) => {
                    if user.is_admin != false {
                        match db::change_event_state(conn, event_id, state) {
                            Ok(_) => show_event(conn, user, event_id, ctx, None, 0),
                            Err(e) => Err(anyhow!("Failed to close event: {}.", e)),
                        }
                    } else {
                        Err(anyhow!("not allowed"))
                    }
                }
                _ => Err(anyhow!("Failed to parse query: {}", data)),
            }
        }
        "show_waiting_list" if pars.len() == 3 => {
            match (pars[1].parse::<u64>(), pars[2].parse::<u64>()) {
                (Ok(event_id), Ok(offset)) => {
                    if ctx.config.public_lists || user.is_admin != false {
                        show_waiting_list(conn, user, event_id, ctx, offset)
                    } else {
                        Err(anyhow!("not allowed"))
                    }
                }
                _ => Err(anyhow!("Failed to parse query: {}", data)),
            }
        }
        "show_presence_list" if pars.len() == 3 => {
            match (pars[1].parse::<u64>(), pars[2].parse::<u64>()) {
                (Ok(event_id), Ok(offset)) => show_presence_list(conn, event_id, user, ctx, offset),
                _ => Err(anyhow!("Failed to parse query: {}", data)),
            }
        }
        "confirm_presence" if pars.len() == 4 => {
            match (
                pars[1].parse::<u64>(),
                pars[2].parse::<u64>(),
                pars[3].parse::<u64>(),
            ) {
                (Ok(event_id), Ok(user_id), Ok(offset)) => {
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
                _ => Err(anyhow!("Failed to parse query: {}", data)),
            }
        }
        _ => Err(anyhow!("Failed to parse query.")),
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
            let mut keyboard: Vec<Vec<InlineKeyboardButton>> = events
                .iter()
                .map(|s| {
                    vec![InlineKeyboardButton::callback(
                        format!(
                            "{} {} / {} / {}",
                            if s.adults.my_reservation != 0 || s.children.my_reservation != 0 {
                                "✅"
                            } else if s.adults.my_waiting != 0 || s.children.my_waiting != 0 {
                                "⏳"
                            } else {
                                ""
                            },
                            format_ts(s.event.ts),
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
                        format!("event {} 0", s.event.id),
                    )]
                })
                .collect();

            let header_text = if offset != 0 || events.len() != 0 {
                format!("Программа\nвремя / взросл.(детск.) места  / мероприятие\n<a href=\"{}\">инструкция</a>", ctx.config.help)
            } else {
                "Нет мероприятий.".to_string()
            };

            add_pagination(
                &mut keyboard,
                "event_list",
                events.len() as u64,
                ctx.config.event_list_page_size,
                offset,
            );

            Ok(Reply::new(header_text).reply_markup(InlineKeyboardMarkup::new(keyboard)))
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
            let mut keyboard: Vec<Vec<InlineKeyboardButton>> = Vec::new();
            let mut v: Vec<InlineKeyboardButton> = Vec::new();
            let free_adults = s.event.max_adults - s.adults.reserved;
            let free_children = s.event.max_children - s.children.reserved;
            let no_age_distinction = s.event.max_adults == 0 || s.event.max_children == 0;
            let is_admin = ctx.admins.contains(&user.id.0);
            if s.state == EventState::Open
                && s.adults.my_reservation < s.event.max_adults_per_reservation
            {
                if free_adults > 0 {
                    v.push(InlineKeyboardButton::callback(
                        if no_age_distinction {
                            "Записаться +1"
                        } else {
                            "Записать взрослого +1"
                        },
                        format!("sign_up {} adult nowait", s.event.id),
                    ));
                } else if s.adults.my_reservation + s.adults.my_waiting
                    < s.event.max_adults_per_reservation
                {
                    v.push(InlineKeyboardButton::callback(
                        if no_age_distinction {
                            "В лист ожидания +1"
                        } else {
                            "В лист ожидания взрослого +1"
                        },
                        format!("sign_up {} adult wait", s.event.id),
                    ));
                }
            }
            if s.adults.my_reservation > 0 || s.adults.my_waiting > 0 {
                v.push(InlineKeyboardButton::callback(
                    if no_age_distinction {
                        "Отписаться -1"
                    } else {
                        "Отписать взрослого -1"
                    },
                    format!("cancel {} adult", s.event.id),
                ));
            }
            keyboard.push(v);
            let mut v: Vec<InlineKeyboardButton> = Vec::new();
            if s.state == EventState::Open
                && s.children.my_reservation < s.event.max_children_per_reservation
            {
                if free_children > 0 {
                    v.push(InlineKeyboardButton::callback(
                        if no_age_distinction {
                            "Записаться +1"
                        } else {
                            "Записать ребёнка +1"
                        },
                        format!("sign_up {} child nowait", s.event.id),
                    ));
                } else if s.children.my_reservation + s.children.my_waiting
                    < s.event.max_children_per_reservation
                {
                    v.push(InlineKeyboardButton::callback(
                        if no_age_distinction {
                            "В лист ожидания +1"
                        } else {
                            "В лист ожидания ребёнка +1"
                        },
                        format!("sign_up {} child wait", s.event.id),
                    ));
                }
            }
            if s.children.my_reservation > 0 || s.children.my_waiting > 0 {
                v.push(InlineKeyboardButton::callback(
                    if no_age_distinction {
                        "Отписаться -1"
                    } else {
                        "Отписать ребёнка -1"
                    },
                    format!("cancel {} child", s.event.id),
                ));
            }
            keyboard.push(v);
            let mut v: Vec<InlineKeyboardButton> = Vec::new();
            v.push(InlineKeyboardButton::callback(
                "Список мероприятий",
                "event_list 0",
            ));
            let mut list = "".to_string();
            let mut participants_len: u64 = 0;
            if ctx.config.public_lists || is_admin {
                match db::get_participants(conn, event_id, 0, offset, ctx.config.event_page_size) {
                    Ok(participants) => {
                        participants_len = participants.len() as u64;
                        if is_admin {
                            list.push_str(&format!(
                                "\nМероприятие {} / {}({})",
                                event_id, s.event.max_adults, s.event.max_children
                            ));
                        }
                        if participants.len() != 0 {
                            list.push_str("\nЗаписались:");
                        }

                        list.push_str(
                            &participants
                                .iter()
                                .map(|p| {
                                    let mut entry = "".to_string();
                                    let id = if is_admin {
                                        p.user_id.to_string() + " "
                                    } else {
                                        "".to_string()
                                    };
                                    if p.user_name2.len() > 0 {
                                        entry.push_str(&format!(
                                            "\n{}<a href=\"https://t.me/{}\">{} ({})</a>",
                                            id, p.user_name2, p.user_name1, p.user_name2
                                        ));
                                    } else {
                                        entry.push_str(&format!(
                                            "\n{}<a href=\"tg://user?id={}\">{}</a>",
                                            id, p.user_id, p.user_name1
                                        ));
                                    }
                                    if no_age_distinction {
                                        entry.push_str(&format!(" {}", p.adults + p.children));
                                    } else {
                                        entry.push_str(&format!(" {} {}", p.adults, p.children));
                                    }
                                    if let Some(a) = &p.attachment {
                                        entry.push_str(&format!(" {}", a));
                                    }
                                    entry
                                })
                                .collect::<String>(),
                        );
                    }
                    Err(e) => {
                        error!("Failed to get participants: {}", e);
                    }
                }
                v.push(InlineKeyboardButton::callback(
                    "Список ожидания",
                    format!("show_waiting_list {} 0", event_id),
                ));
                if is_admin {
                    v.push(InlineKeyboardButton::callback(
                        "Присутствие",
                        format!("show_presence_list {} 0", event_id),
                    ));
                    if s.state == EventState::Open {
                        v.push(InlineKeyboardButton::callback(
                            "Остановить запись",
                            format!("change_event_state {} 1", event_id),
                        ));
                    } else {
                        v.push(InlineKeyboardButton::callback(
                            "Разрешить запись",
                            format!("change_event_state {} 0", event_id),
                        ));
                    }
                } else {
                    if let Ok(check) = db::is_group_leader(conn, event_id, user.id.0) {
                        if check {
                            v.push(InlineKeyboardButton::callback(
                                "Присутствие",
                                format!("show_presence_list {} 0", event_id),
                            ));
                        }
                    }
                }
            }
            keyboard.push(v);
            add_pagination(
                &mut keyboard,
                &format!("event {}", event_id),
                participants_len,
                ctx.config.event_page_size,
                offset,
            );

            let mut text = format!(
                "\n \n{}\nНачало: {}.",
                format_event_title(&s.event),
                format_ts(s.event.ts)
            );
            if s.state == EventState::Open {
                if no_age_distinction {
                    text.push_str(&format!(
                        " Свободные места: {}",
                        free_adults + free_children,
                    ));
                } else {
                    text.push_str(&format!("\nВзрослые свободные места: {}", free_adults));
                    text.push_str(&format!("\nДетские свободные места: {}", free_children));
                }
            } else {
                text.push_str(" Запись остановлена.");
            }
            text.push_str(&format!("\n{}\n", list));
            if is_admin
                || s.adults.my_reservation > 0
                || s.adults.my_waiting > 0
                || s.children.my_reservation > 0
                || s.children.my_waiting > 0
            {
                if let Ok(messages) = db::get_messages(
                    conn,
                    event_id,
                    if is_admin {
                        None
                    } else {
                        Some(
                            (s.adults.my_reservation == 0 && s.children.my_reservation == 0) as u64,
                        )
                    },
                ) {
                    if let Some(messages) = messages {
                        text.push_str(&format!(
                            "\n<b>Cообщения по мероприятию</b>\n{}\n",
                            messages
                        ));
                    }
                }

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
            }
            if let Some(ps) = ps {
                text.push_str(&ps);
            }

            Ok(Reply::new(text).reply_markup(InlineKeyboardMarkup::new(keyboard)))
        }
        Err(e) => Err(anyhow!("Failed to fetch event: {}", e)),
    }
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
    let is_admin = ctx.admins.contains(&user.id.0);
    match db::get_event(conn, event_id, user.id.0) {
        Ok(s) => {
            no_age_distinction = s.event.max_adults == 0 || s.event.max_children == 0;
            list.push_str(&format!(
                "\n \n{}\nНачало: {}\n",
                format_event_title(&s.event),
                format_ts(s.event.ts)
            ));
        }
        Err(e) => {
            return Err(anyhow!("Failed to find event: {}", e));
        }
    }
    match db::get_participants(conn, event_id, 1, offset, ctx.config.event_page_size) {
        Ok(participants) => {
            let mut keyboard: Vec<Vec<InlineKeyboardButton>> = Vec::new();
            add_pagination(
                &mut keyboard,
                &format!("show_waiting_list {}", event_id),
                participants.len() as u64,
                ctx.config.event_page_size,
                offset,
            );

            let mut v: Vec<InlineKeyboardButton> = Vec::new();
            v.push(InlineKeyboardButton::callback(
                "Назад",
                format!("event {} 0", event_id),
            ));
            keyboard.push(v);

            if participants.len() == 0 {
                list.push_str("Пустой список ожидания.");
            } else {
                list.push_str("Список ожидания:");
                list.push_str(
                    &participants
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
                                entry.push_str(&format!(" {} {}", p.adults, p.children));
                            }
                            if let Some(a) = &p.attachment {
                                entry.push_str(&format!(" {}", a));
                            }
                            entry
                        })
                        .collect::<String>(),
                );
            }

            Ok(Reply::new(list).reply_markup(InlineKeyboardMarkup::new(keyboard)))
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
                format_event_title(&s.event),
                format_ts(s.event.ts)
            ));
        }
        Err(e) => {
            return Err(anyhow!("Failed to find event: {}", e));
        }
    }
    match db::get_presence_list(conn, event_id, offset, ctx.config.presence_page_size) {
        Ok(participants) => {
            if participants.len() != 0 {
                header.push_str("Пожалуйста, выберите присутствующих:");
            }

            let mut keyboard: Vec<Vec<InlineKeyboardButton>> = participants
                .iter()
                .map(|p| {
                    vec![{
                        let mut text;
                        if p.user_name2.len() > 0 {
                            text = format!("{} ({}) {}", p.user_name1, p.user_name2, p.reserved);
                        } else {
                            text = format!("{} {}", p.user_name1, p.reserved);
                        }
                        if let Some(a) = &p.attachment {
                            text.push_str(&format!(" - {}", a));
                        }

                        InlineKeyboardButton::callback(
                            text,
                            format!("confirm_presence {} {} {}", event_id, p.user_id, offset),
                        )
                    }]
                })
                .collect();

            add_pagination(
                &mut keyboard,
                &format!("show_presence_list {}", event_id),
                participants.len() as u64,
                ctx.config.presence_page_size,
                offset,
            );

            let mut v: Vec<InlineKeyboardButton> = Vec::new();
            v.push(InlineKeyboardButton::callback(
                "Назад",
                format!("event {} 0", event_id),
            ));
            keyboard.push(v);

            Ok(Reply::new(header).reply_markup(InlineKeyboardMarkup::new(keyboard)))
        }
        Err(e) => Err(anyhow!("Failed to get precense list: {}", e)),
    }
}

pub fn add_pagination(
    keyboard: &mut Vec<Vec<InlineKeyboardButton>>,
    cmd: &str,
    participants: u64,
    limit: u64,
    offset: u64,
) {
    if offset > 0 || participants == limit {
        let mut pagination: Vec<InlineKeyboardButton> = Vec::new();
        if offset > 0 {
            pagination.push(InlineKeyboardButton::callback(
                "⬅️",
                format!("{} {}", cmd, offset - 1),
            ));
        }
        if participants == limit {
            pagination.push(InlineKeyboardButton::callback(
                "➡️",
                format!("{} {}", cmd, offset + 1),
            ));
        }
        keyboard.push(pagination);
    }
}
