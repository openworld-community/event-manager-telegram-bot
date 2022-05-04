use crate::db;
use crate::format_ts;
use std::collections::HashSet;
use telegram_bot::{Api, CanEditMessageText, CanSendMessage, MessageOrChannelPost};

pub fn show_event_list(
    db: &db::EventDB,
    api: &Api,
    user_id: telegram_bot::UserId,
    callback: Option<MessageOrChannelPost>,
) {
    match db.get_events(user_id.into()) {
        Ok(events) => {
            if events.len() > 0 {
                let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                for s in events {
                    let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                    v.push(telegram_bot::types::InlineKeyboardButton::callback(
                        format!(
                            "{} /{}({})/ {}",
                            format_ts(s.event.ts),
                            s.event.max_adults - s.adults,
                            s.event.max_children - s.children,
                            s.event.name
                        ),
                        format!("event {}", s.event.id),
                    ));
                    keyboard.add_row(v);
                }

                let text = "Программа\nвремя / взросл.(детск.) места  / мероприятие";

                if let Some(msg) = callback {
                    if let MessageOrChannelPost::Message(msg) = msg {
                        api.spawn(
                            msg.edit_text(text)
                                .parse_mode(telegram_bot::types::ParseMode::Html)
                                .disable_preview()
                                .reply_markup(keyboard),
                        );
                    }
                } else {
                    api.spawn(user_id.text(text).reply_markup(keyboard));
                }
            } else {
                api.spawn(user_id.text("Нет мероприятий."));
            }
        }
        Err(e) => {
            api.spawn(user_id.text(format!("Failed to query events: {}", e.to_string())));
        }
    }
}

pub fn show_event(
    db: &db::EventDB,
    api: &Api,
    user_id: telegram_bot::UserId,
    event_id: i64,
    admin_ids: &HashSet<i64>,
    admin_names: &HashSet<String>,
    user_name2: &str,
    callback: Option<MessageOrChannelPost>,
    public_lists: bool,
) {
    match db.get_event(event_id, user_id.into()) {
        Ok(s) => {
            let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
            let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
            let free_adults = s.event.max_adults - s.adults;
            let free_children = s.event.max_children - s.children;
            if s.my_adults < s.event.max_adults_per_reservation {
                if free_adults > 0 {
                    v.push(telegram_bot::types::InlineKeyboardButton::callback(
                        "Записать взрослого",
                        format!("sign_up {} adult nowait", s.event.id),
                    ));
                } else {
                    v.push(telegram_bot::types::InlineKeyboardButton::callback(
                        "В лист ожидания взрослого",
                        format!("sign_up {} adult wait", s.event.id),
                    ));
                }
            }
            if s.my_adults > 0 || s.my_wait_adults > 0 {
                v.push(telegram_bot::types::InlineKeyboardButton::callback(
                    "Отписать взрослого",
                    format!("cancel {} adult", s.event.id),
                ));
            }
            keyboard.add_row(v);

            let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
            if s.my_children < s.event.max_children_per_reservation {
                if free_children > 0 {
                    v.push(telegram_bot::types::InlineKeyboardButton::callback(
                        "Записать ребёнка",
                        format!("sign_up {} child nowait", s.event.id),
                    ));
                } else {
                    v.push(telegram_bot::types::InlineKeyboardButton::callback(
                        "В лист ожидания ребёнка",
                        format!("sign_up {} child wait", s.event.id),
                    ));
                }
            }
            if s.my_children > 0 || s.my_wait_children > 0 {
                v.push(telegram_bot::types::InlineKeyboardButton::callback(
                    "Отписать ребёнка",
                    format!("cancel {} child", s.event.id),
                ));
            }
            keyboard.add_row(v);

            let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
            v.push(telegram_bot::types::InlineKeyboardButton::callback(
                "Список мероприятий",
                "event_list",
            ));

            let is_admin = admin_ids.contains(&user_id.into()) != false || admin_names.contains(user_name2) != false;
            let mut list = "".to_string();
            if public_lists || is_admin {
                match db.get_participants(event_id, 0) {
                    Ok(participants) => {
                        if participants.len() != 0 {
                            if is_admin {
                                list.push_str(&format!("\nМероприятие {}", event_id));
                            }
                            list.push_str("\nЗаписались:");
                        }
                        for p in participants {
                            if p.user_name2.len() > 0 {
                                list.push_str(&format!(
                                    "\n<a href=\"https://t.me/{}\">{} ({})</a>",
                                    p.user_name2, p.user_name1, p.user_name2
                                ));
                            } else {
                                list.push_str(&format!(
                                    "\n<a href=\"tg://user?id={}\">{}</a>",
                                    p.user_id, p.user_name1
                                ));
                            }
                            list.push_str(&format!(" {} {}", p.adults, p.children));
                            if let Some(a) = p.attachment {
                                list.push_str(&format!(" {}", a));
                            }
                        }
                    }
                    Err(_) => {}
                }

                v.push(telegram_bot::types::InlineKeyboardButton::callback(
                    "Список ожидания",
                    format!("show_waiting_list {}", event_id),
                ));
                if is_admin {
                    v.push(telegram_bot::types::InlineKeyboardButton::callback(
                        "Удалить мероприятие",
                        format!("delete {}", event_id),
                    ));
                }
            }
            keyboard.add_row(v);

            let mut text = format!("\n \n<a href=\"{}\">{}</a>\nНачало: {}\nВзрослые места: свободные - {}, моя бронь - {}", s.event.link, s.event.name, format_ts(s.event.ts), free_adults, s.my_adults);
            if s.my_wait_adults > 0 {
                text.push_str(&format!(", лист ожидания - {}", s.my_wait_adults));
            }
            text.push_str(&format!(
                "\nДетские места: свободные - {}, моя бронь - {}",
                free_children, s.my_children
            ));
            if s.my_wait_children > 0 {
                text.push_str(&format!(", лист ожидания - {}", s.my_wait_children));
            }
            text.push_str(&format!("\n{}\n", list));

            if s.my_adults > 0
                || s.my_wait_adults > 0
                || s.my_children > 0
                || s.my_wait_children > 0
            {
                if public_lists == false {
                    match db.get_attachment(event_id, user_id.into()) {
                        Ok(v) => match v {
                            Some(attachment) => {
                                text.push_str(&format!("\nПримечание: {}.", attachment));
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
                text.push_str("\nПошлите сообщение этому боту чтобы добавить примечание к брони. (Макс. 256 символов)");
            }

            if let Some(msg) = callback {
                if let MessageOrChannelPost::Message(msg) = msg {
                    api.spawn(
                        msg.edit_text(text)
                            .parse_mode(telegram_bot::types::ParseMode::Html)
                            .disable_preview()
                            .reply_markup(keyboard),
                    );
                }
            } else {
                api.spawn(
                    user_id
                        .text(text)
                        .parse_mode(telegram_bot::types::ParseMode::Html)
                        .disable_preview()
                        .reply_markup(keyboard),
                );
            }
        }
        Err(_e) => {}
    }
}

pub fn show_waiting_list(
    db: &db::EventDB,
    api: &Api,
    user_id: telegram_bot::UserId,
    event_id: i64,
    callback: Option<MessageOrChannelPost>,
) {
    match db.get_participants(event_id, 1) {
        Ok(participants) => {
            let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
            let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
            v.push(telegram_bot::types::InlineKeyboardButton::callback(
                "Назад",
                format!("event {}", event_id),
            ));
            keyboard.add_row(v);

            let mut list = "".to_string();
            if participants.len() == 0 {
                list.push_str("Пустой список ожидания.");
            } else {
                list.push_str("Список ожидания:");
                for p in participants {
                    if p.user_name2.len() > 0 {
                        list.push_str(&format!(
                            "\n<a href=\"https://t.me/{}\">{} ({})</a>",
                            p.user_name2, p.user_name1, p.user_name2
                        ));
                    } else {
                        list.push_str(&format!(
                            "\n<a href=\"tg://user?id={}\">{}</a>",
                            p.user_id, p.user_name1
                        ));
                    }
                    list.push_str(&format!(" {} {}", p.adults, p.children));
                    if let Some(a) = p.attachment {
                        list.push_str(&format!(" {}", a));
                    }
                }
            }

            if let Some(msg) = callback {
                if let MessageOrChannelPost::Message(msg) = msg {
                    api.spawn(
                        msg.edit_text(list)
                            .parse_mode(telegram_bot::types::ParseMode::Html)
                            .disable_preview()
                            .reply_markup(keyboard),
                    );
                }
            } else {
                api.spawn(
                    user_id
                        .text(list)
                        .parse_mode(telegram_bot::types::ParseMode::Html)
                        .disable_preview()
                        .reply_markup(keyboard),
                );
            }
        }
        Err(_e) => {}
    }
}

pub fn notify_users_on_waiting_list(api: &Api, event_id: i64, update: HashSet<i64>) {
    let text = "Одно из ваших бронирований в списке ожидания подтверждено.";
    let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
    let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
    v.push(telegram_bot::types::InlineKeyboardButton::callback(
        "Посмотреть",
        format!("event {}", event_id),
    ));
    keyboard.add_row(v);

    for user_id in update {
        api.spawn(
            telegram_bot::types::UserId::new(user_id)
                .text(text)
                .reply_markup(keyboard.clone()),
        );
    }
}
