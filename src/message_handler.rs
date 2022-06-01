use crate::db::{EventDB, EventState};
use crate::format_ts;
use crate::util::*;
use std::collections::{HashMap, HashSet};
use telegram_bot::{Api, CanEditMessageText, CanSendMessage, MessageOrChannelPost, UserId};

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Configuration {
    pub telegram_bot_token: String,
    pub admin_ids: String,
    pub public_lists: bool,
    pub automatic_blacklisting: bool,
    pub drop_events_after_hours: i64,
    pub delete_from_black_list_after_days: i64,
    pub too_late_to_cancel_hours: i64,
    pub perform_periodic_tasks: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct User {
    pub id: UserId,
    pub user_name1: String,
    pub user_name2: String,
    pub is_admin: bool,
}

impl User {
    pub fn new(u: telegram_bot::User, admins: &HashSet<i64>) -> User {
        let mut user_name1 = u.first_name.clone();
        if let Some(v) = u.last_name.clone() {
            user_name1.push_str(" ");
            user_name1.push_str(&v);
        }
        let user_name2 = match u.username.clone() {
            Some(name) => name,
            None => "".to_string(),
        };

        User {
            id: u.id,
            user_name1,
            user_name2: user_name2.clone(),
            is_admin: admins.contains(&u.id.into()),
        }
    }
}

pub struct MessageHandler<'a> {
    db: &'a EventDB,
    api: &'a Api,
    config: &'a Configuration,
}

impl<'a> MessageHandler<'a> {
    pub fn new(db: &'a EventDB, api: &'a Api, config: &'a Configuration) -> MessageHandler<'a> {
        MessageHandler { db, api, config }
    }

    pub async fn process_message(
        &self,
        user: &User,
        data: &str,
        active_events: &mut HashMap<i64, i64>,
    ) -> bool {
        let pars: Vec<&str> = data.splitn(3, ' ').collect();
        if pars.len() == 0 {
            return false;
        }
        match pars[0] {
            "/start" => {
                if pars.len() == 2 {
                    // Direct link
                    if let Ok(event_id) = pars[1].parse::<i64>() {
                        self.show_event(user, event_id, &None, None);
                    }
                } else {
                    self.show_event_list(user.id, &None);
                }
            }
            "/help" => {
                self.api.spawn(
                    user.id
                        .text(
                            "Здесь вы можете бронировать места на мероприятия.\n \
                            \n /start - показать список мероприятий \
                            \n /help - эта подсказка",
                        )
                        .disable_preview(),
                );
            }
            _ => {
                // Message from user - try to add as attachment to the last reservation.
                self.add_attachment(&user, data, active_events);
            }
        }

        true
    }

    pub fn process_query(
        &self,
        user: &User,
        data: &str,
        message: &Option<MessageOrChannelPost>,
        active_events: &mut HashMap<i64, i64>,
    ) -> bool {
        let pars: Vec<&str> = data.splitn(4, ' ').collect();
        if pars.len() == 0 {
            return false;
        }
        match pars[0] {
            "event_list" => {
                self.show_event_list(user.id, message);
            }
            "event" if pars.len() == 2 => match pars[1].parse::<i64>() {
                Ok(event_id) => {
                    active_events.insert(user.id.into(), event_id);
                    self.show_event(user, event_id, message, None);
                }
                Err(_e) => {}
            },
            "sign_up" if pars.len() == 4 => match pars[1].parse::<i64>() {
                Ok(event_id) => {
                    let is_adult = pars[2] == "adult";
                    let wait = pars[3] == "wait";
                    match self.db.sign_up(
                        event_id,
                        user.id.into(),
                        &user.user_name1,
                        &user.user_name2,
                        is_adult as i64,
                        !is_adult as i64,
                        wait as i64,
                        get_unix_time(),
                    ) {
                        Ok((_, black_listed)) => {
                            self.show_event(
                                user,
                                event_id,
                                message,
                                if black_listed {
                                    Some("\n\nИзвините, но бронирование невозможно, поскольку ранее Вы не использовали и не отменили бронь. Если это ошибка, пожалуйста, свяжитесь с администратором.".to_string())
                                } else {
                                    None
                                },
                            );
                        }
                        Err(e) => {
                            self.api.spawn(user.id.text(format!("{}", e)));
                        }
                    }
                }
                Err(_e) => {}
            },
            "cancel" if pars.len() == 3 => {
                if let Ok(event_id) = pars[1].parse::<i64>() {
                    if self.is_too_late_to_cancel(event_id, user) {
                        self.api
                            .spawn(user.id.text("Извините, отменить бронь уже невозможно."));
                    } else {
                        let is_adult = pars[2] == "adult";
                        match self.db.cancel(event_id, user.id.into(), is_adult as i64) {
                            Ok(update) => {
                                self.show_event(user, event_id, message, None);
                                self.notify_users_on_waiting_list(event_id, update);
                            }
                            Err(e) => {
                                self.api
                                    .spawn(user.id.text(format!("Failed to add event: {}.", e)));
                            }
                        }
                    }
                }
            }
            "wontgo" if pars.len() == 2 => match pars[1].parse::<i64>() {
                Ok(event_id) => match self.db.wontgo(event_id, user.id.into()) {
                    Ok(update) => {
                        if self.is_too_late_to_cancel(event_id, user) {
                            self.api
                                .spawn(user.id.text("Извините, отменить бронь уже невозможно."));
                        } else {
                            self.api.spawn(user.id.text("Мы сожалеем, что вы не сможете пойти. Увидимся в другой раз. Спасибо!"));
                            self.notify_users_on_waiting_list(event_id, update);
                        }
                    }
                    Err(e) => {
                        self.api
                            .spawn(user.id.text(format!("Failed to add event: {}.", e)));
                    }
                },
                Err(_e) => {}
            },
            "change_event_state" if pars.len() == 3 => {
                match (pars[1].parse::<i64>(), pars[2].parse::<i64>()) {
                    (Ok(event_id), Ok(state)) => {
                        if user.is_admin != false {
                            match self.db.change_event_state(event_id, state) {
                                Ok(_) => {
                                    self.show_event(user, event_id, &None, None);
                                }
                                Err(e) => {
                                    self.api.spawn(
                                        user.id.text(format!("Failed to close event: {}.", e)),
                                    );
                                }
                            }
                        } else {
                            warn!("not allowed");
                        }
                    }
                    _ => {}
                }
            }
            "show_waiting_list" if pars.len() == 2 => match pars[1].parse::<i64>() {
                Ok(event_id) => {
                    if self.config.public_lists || user.is_admin != false {
                        self.show_waiting_list(user, event_id, message);
                    } else {
                        warn!("not allowed");
                    }
                }
                Err(_e) => {}
            },
            "show_presence_list" if pars.len() == 2 => match pars[1].parse::<i64>() {
                Ok(event_id) => {
                    self.show_presence_list(event_id, user, message);
                }
                Err(_) => {}
            },
            "confirm_presence" if pars.len() == 3 => {
                match (pars[1].parse::<i64>(), pars[2].parse::<i64>()) {
                    (Ok(event_id), Ok(user_id)) => {
                        let user_has_permissions = if user.is_admin {
                            true
                        } else {
                            match self.db.is_group_leader(event_id, user.id.into()) {
                                Ok(res) => res,
                                Err(_) => false,
                            }
                        };
                        if user_has_permissions {
                            match self.db.confirm_presence(event_id, user_id) {
                                Ok(_) => {
                                    self.show_presence_list(event_id, user, message);
                                }
                                Err(e) => {
                                    self.api.spawn(
                                        user.id.text(format!("Failed to confirm presence: {}.", e)),
                                    );
                                }
                            }
                        } else {
                            self.api.spawn(user.id.text(format!("Not allowed.")));
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                self.api.spawn(user.id.text("Faied to parse query."));
                return false;
            }
        }

        true
    }

    pub fn add_attachment(&self, user: &User, data: &str, active_events: &mut HashMap<i64, i64>) {
        let user_id: i64 = user.id.into();
        let event_id = match active_events.get(&user_id) {
            Some(v) => *v,
            _ => match self.db.get_last_reservation_event(user_id) {
                Ok(v) => v,
                _ => 0,
            },
        };
        if event_id != 0 {
            match self.db.add_attachment(event_id, user_id, data) {
                Ok(_v) => {
                    self.show_event(
                        user,
                        event_id,
                        &None,
                        if data.chars().any(char::is_numeric) {
                            Some("\n\nВНИМАНИЕ!\nВаше примечание содержит цифры. Они никак не влияют на количество забронированных мест. Количество мест можно менять только кнопками \"Записать/Отписать\".".to_string())
                        } else {
                            None
                        },
                    );
                }
                _ => {}
            }
        }
    }

    pub fn show_event_list(
        &self,
        user_id: telegram_bot::UserId,
        callback: &Option<MessageOrChannelPost>,
    ) {
        match self.db.get_events(user_id.into()) {
            Ok(events) => {
                if events.len() > 0 {
                    let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                    for s in &events {
                        let marker =
                            if s.adults.my_reservation != 0 || s.children.my_reservation != 0 {
                                "✅"
                            } else if s.adults.my_waiting != 0 || s.children.my_waiting != 0 {
                                "⏳"
                            } else {
                                ""
                            };

                        let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            format!(
                                "{} {} / {} / {}",
                                marker,
                                format_ts(s.event.ts),
                                if s.state == EventState::Open {
                                    if s.event.max_adults == 0 || s.event.max_children == 0 {
                                        (s.event.max_adults - s.adults.reserved + s.event.max_children - s.children.reserved).to_string()
                                    }
                                    else {
                                        format!("{}({})", s.event.max_adults - s.adults.reserved, s.event.max_children - s.children.reserved)
                                    }
                                } else {
                                    "-".to_string()
                                },
                                s.event.name
                            ),
                            format!("event {}", s.event.id),
                        ));
                        keyboard.add_row(v);
                    }
                    let header_text = "Программа\nвремя / взросл.(детск.) места  / мероприятие";
                    if let Some(msg) = callback {
                        if let MessageOrChannelPost::Message(msg) = msg {
                            self.api.spawn(
                                msg.edit_text(header_text)
                                    .parse_mode(telegram_bot::types::ParseMode::Html)
                                    .disable_preview()
                                    .reply_markup(keyboard),
                            );
                        }
                    } else {
                        self.api
                            .spawn(user_id.text(header_text).reply_markup(keyboard));
                    }
                } else {
                    self.api.spawn(user_id.text("Нет мероприятий."));
                }
            }
            Err(e) => {
                self.api
                    .spawn(user_id.text(format!("Failed to query events: {}", e.to_string())));
            }
        }
    }
    pub fn show_event(
        &self,
        user: &User,
        event_id: i64,
        callback: &Option<MessageOrChannelPost>,
        ps: Option<String>,
    ) {
        match self.db.get_event(event_id, user.id.into()) {
            Ok(s) => {
                let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                let free_adults = s.event.max_adults - s.adults.reserved;
                let free_children = s.event.max_children - s.children.reserved;
                let no_age_distinction = s.event.max_adults == 0 || s.event.max_children == 0;
                if s.state == EventState::Open
                    && s.adults.my_reservation + s.adults.my_waiting
                        < s.event.max_adults_per_reservation
                {
                    if free_adults > 0 {
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            if no_age_distinction {
                                "Записаться +1"
                            } else {
                                "Записать взрослого +1"
                            },
                            format!("sign_up {} adult nowait", s.event.id),
                        ));
                    } else {
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
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
                    v.push(telegram_bot::types::InlineKeyboardButton::callback(
                        if no_age_distinction {
                            "Отписаться -1"
                        } else {
                            "Отписать взрослого -1"
                        },
                        format!("cancel {} adult", s.event.id),
                    ));
                }
                keyboard.add_row(v);
                let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                if s.state == EventState::Open
                    && s.children.my_reservation + s.children.my_waiting
                        < s.event.max_children_per_reservation
                {
                    if free_children > 0 {
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            if no_age_distinction {
                                "Записаться +1"
                            } else {
                                "Записать ребёнка +1"
                            },
                            format!("sign_up {} child nowait", s.event.id),
                        ));
                    } else {
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
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
                    v.push(telegram_bot::types::InlineKeyboardButton::callback(
                        if no_age_distinction {
                            "Отписаться -1"
                        } else {
                            "Отписать ребёнка -1"
                        },
                        format!("cancel {} child", s.event.id),
                    ));
                }
                keyboard.add_row(v);
                let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                v.push(telegram_bot::types::InlineKeyboardButton::callback(
                    "Список мероприятий",
                    "event_list",
                ));
                let mut list = "".to_string();
                if self.config.public_lists || user.is_admin {
                    match self.db.get_participants(event_id, 0) {
                        Ok(participants) => {
                            if user.is_admin {
                                list.push_str(&format!("\nМероприятие {}", event_id));
                            }
                            if participants.len() != 0 {
                                list.push_str("\nЗаписались:");
                            }
                            for p in &participants {
                                let id = if user.is_admin {
                                    p.user_id.to_string() + " "
                                } else {
                                    "".to_string()
                                };
                                if p.user_name2.len() > 0 {
                                    list.push_str(&format!(
                                        "\n{}<a href=\"https://t.me/{}\">{} ({})</a>",
                                        id, p.user_name2, p.user_name1, p.user_name2
                                    ));
                                } else {
                                    list.push_str(&format!(
                                        "\n{}<a href=\"tg://user?id={}\">{}</a>",
                                        id, p.user_id, p.user_name1
                                    ));
                                }
                                if no_age_distinction {
                                    list.push_str(&format!(" {}", p.adults + p.children));
                                } else {
                                    list.push_str(&format!(" {} {}", p.adults, p.children));
                                }
                                if let Some(a) = &p.attachment {
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
                    if user.is_admin {
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            "Присутствие",
                            format!("show_presence_list {}", event_id),
                        ));
                        if s.state == EventState::Open {
                            v.push(telegram_bot::types::InlineKeyboardButton::callback(
                                "Остановить запись",
                                format!("change_event_state {} 1", event_id),
                            ));
                        } else {
                            v.push(telegram_bot::types::InlineKeyboardButton::callback(
                                "Разрешить запись",
                                format!("change_event_state {} 0", event_id),
                            ));
                        }
                    } else {
                        if let Ok(check) = self.db.is_group_leader(event_id, user.id.into()) {
                            if check {
                                v.push(telegram_bot::types::InlineKeyboardButton::callback(
                                    "Присутствие",
                                    format!("show_presence_list {}", event_id),
                                ));
                            }
                        }
                    }
                }
                keyboard.add_row(v);
                let mut text = format!(
                    "\n \n<a href=\"{}\">{}</a>\nНачало: {}.",
                    s.event.link,
                    s.event.name,
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
                if user.is_admin
                    || s.adults.my_reservation > 0
                    || s.adults.my_waiting > 0
                    || s.children.my_reservation > 0
                    || s.children.my_waiting > 0
                {
                    if let Ok(messages) = self.db.get_messages(
                        event_id,
                        if user.is_admin { None } else { Some((s.adults.my_reservation == 0 && s.children.my_reservation == 0) as i64)},
                    ) {
                        if let Some(messages) = messages {
                            text.push_str(&format!(
                                "\n<b>Cообщения по мероприятию</b>\n{}\n",
                                messages
                            ));
                        }
                    }

                    if self.config.public_lists == false {
                        match self.db.get_attachment(event_id, user.id.into()) {
                            Ok(v) => match v {
                                Some(attachment) => {
                                    text.push_str(&format!("\nПримечание: {}.", attachment));
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                    if user.is_admin == false {
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
                if let Some(msg) = callback {
                    if let MessageOrChannelPost::Message(msg) = msg {
                        self.api.spawn(
                            msg.edit_text(text)
                                .parse_mode(telegram_bot::types::ParseMode::Html)
                                .disable_preview()
                                .reply_markup(keyboard),
                        );
                    }
                } else {
                    self.api.spawn(
                        user.id
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
    fn show_waiting_list(
        &self,
        user: &User,
        event_id: i64,
        callback: &Option<MessageOrChannelPost>,
    ) {
        let mut list = "".to_string();
        let no_age_distinction;
        match self.db.get_event(event_id, user.id.into()) {
            Ok(s) => {
                no_age_distinction = s.event.max_adults == 0 || s.event.max_children == 0;
                list.push_str(&format!(
                    "\n \n<a href=\"{}\">{}</a>\nНачало: {}\n",
                    s.event.link,
                    s.event.name,
                    format_ts(s.event.ts)
                ));
            }
            Err(_e) => {
                self.api.spawn(user.id.text("Failed to find event"));
                return;
            }
        }
        match self.db.get_participants(event_id, 1) {
            Ok(participants) => {
                let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                v.push(telegram_bot::types::InlineKeyboardButton::callback(
                    "Назад",
                    format!("event {}", event_id),
                ));
                keyboard.add_row(v);
                if participants.len() == 0 {
                    list.push_str("Пустой список ожидания.");
                } else {
                    list.push_str("Список ожидания:");
                    for p in &participants {
                        let id = if user.is_admin {
                            p.user_id.to_string()
                        } else {
                            "".to_string()
                        };
                        if p.user_name2.len() > 0 {
                            list.push_str(&format!(
                                "\n{} <a href=\"https://t.me/{}\">{} ({})</a>",
                                id, p.user_name2, p.user_name1, p.user_name2
                            ));
                        } else {
                            list.push_str(&format!(
                                "\n{} <a href=\"tg://user?id={}\">{}</a>",
                                id, p.user_id, p.user_name1
                            ));
                        }
                        if no_age_distinction {
                            list.push_str(&format!(" {}", p.adults + p.children));
                        } else {
                            list.push_str(&format!(" {} {}", p.adults, p.children));
                        }
                        if let Some(a) = &p.attachment {
                            list.push_str(&format!(" {}", a));
                        }
                    }
                }
                if let Some(msg) = callback {
                    if let MessageOrChannelPost::Message(msg) = msg {
                        self.api.spawn(
                            msg.edit_text(list)
                                .parse_mode(telegram_bot::types::ParseMode::Html)
                                .disable_preview()
                                .reply_markup(keyboard),
                        );
                    }
                } else {
                    self.api.spawn(
                        user.id
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
    pub fn notify_users_on_waiting_list(&self, event_id: i64, update: HashSet<i64>) {
        let text = "Одно из ваших бронирований в списке ожидания подтверждено. Если вы не сможете пойти, отпишитесь, пожалуйста, чтобы дать возможность следующим в списке ожидания. Спасибо!";
        let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
        let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
        v.push(telegram_bot::types::InlineKeyboardButton::callback(
            "К мероприятию",
            format!("event {}", event_id),
        ));
        keyboard.add_row(v);
        for user_id in update {
            self.api.spawn(
                telegram_bot::types::UserId::new(user_id)
                    .text(text)
                    .reply_markup(keyboard.clone()),
            );
        }
    }

    fn is_too_late_to_cancel(&self, event_id: i64, user: &User) -> bool {
        if let Ok(s) = self.db.get_event(event_id, user.id.into()) {
            if s.event.ts - get_unix_time() < self.config.too_late_to_cancel_hours * 60 * 60 {
                return true;
            }
        }
        false
    }

    fn show_presence_list(
        &self,
        event_id: i64,
        user: &User,
        callback: &Option<MessageOrChannelPost>,
    ) {
        let mut header = "".to_string();
        match self.db.get_event(event_id, user.id.into()) {
            Ok(s) => {
                header.push_str(&format!(
                    "\n \n<a href=\"{}\">{}</a>\nНачало: {}\n",
                    s.event.link,
                    s.event.name,
                    format_ts(s.event.ts)
                ));
            }
            Err(_e) => {
                self.api.spawn(user.id.text("Failed to find event"));
                return;
            }
        }
        match self.db.get_presence_list(event_id) {
            Ok(participants) => {
                let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                if participants.len() != 0 {
                    header.push_str("Пожалуйста, выберите присутствующих:");
                    for p in &participants {
                        let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                        let mut text;
                        if p.user_name2.len() > 0 {
                            text = format!("{} ({}) {}", p.user_name1, p.user_name2, p.reserved);
                        } else {
                            text = format!("{} {}", p.user_name1, p.reserved);
                        }
                        if let Some(a) = &p.attachment {
                            text.push_str(&format!(" - {}", a));
                        }

                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            text,
                            format!("confirm_presence {} {}", event_id, p.user_id),
                        ));

                        keyboard.add_row(v);
                    }
                }
                let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                v.push(telegram_bot::types::InlineKeyboardButton::callback(
                    "Назад",
                    format!("event {}", event_id),
                ));
                keyboard.add_row(v);

                if let Some(msg) = callback {
                    if let MessageOrChannelPost::Message(msg) = msg {
                        self.api.spawn(
                            msg.edit_text(header)
                                .parse_mode(telegram_bot::types::ParseMode::Html)
                                .disable_preview()
                                .reply_markup(keyboard),
                        );
                    }
                } else {
                    self.api.spawn(
                        user.id
                            .text(header)
                            .parse_mode(telegram_bot::types::ParseMode::Html)
                            .disable_preview()
                            .reply_markup(keyboard),
                    );
                }
            }
            Err(_e) => {}
        }
    }
}
