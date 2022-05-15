use crate::db::EventDB;
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
    pub db: &'a EventDB,
    pub api: &'a Api,
    pub config: &'a Configuration,
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
        // Direct link
        if let Some(v) = data.find("/start ") {
            if v == 0 {
                let pars: Vec<&str> = data.split(' ').collect();
                if pars.len() == 2 {
                    if let Ok(event_id) = pars[1].parse::<i64>() {
                        self.show_event(user, event_id, &None, None);
                    }
                }
            }
        } else if data == "/start" {
            self.show_event_list(user.id, &None);
        } else {
            // Message from user - try to add as attachment to the last reservation.
            self.add_attachment(&user, data, active_events);
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
        if data == "event_list" {
            self.show_event_list(user.id, message);
        } else if data.find("event ").is_some() {
            let pars: Vec<&str> = data.split(' ').collect();
            if pars.len() == 2 {
                match pars[1].parse::<i64>() {
                    Ok(event_id) => {
                        active_events.insert(user.id.into(), event_id);
                        self.show_event(user, event_id, message, None);
                    }
                    Err(_e) => {}
                }
            }
        } else if data.find("sign_up ").is_some() {
            let pars: Vec<&str> = data.split(' ').collect();
            if pars.len() == 4 {
                match pars[1].parse::<i64>() {
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
                                        Some("\n\nВы добавлены в список ожидания.".to_string())
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
                }
            }
        } else if data.find("cancel ").is_some() {
            let pars: Vec<&str> = data.split(' ').collect();
            if pars.len() == 3 {
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
        } else if data.find("wontgo ").is_some() {
            let pars: Vec<&str> = data.split(' ').collect();
            if pars.len() == 2 {
                match pars[1].parse::<i64>() {
                    Ok(event_id) => match self.db.wontgo(event_id, user.id.into()) {
                        Ok(update) => {
                            if self.is_too_late_to_cancel(event_id, user) {
                                self.api.spawn(
                                    user.id.text("Извините, отменить бронь уже невозможно."),
                                );
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
                }
            }
        } else if data.find("change_event_state ").is_some() {
            let pars: Vec<&str> = data.split(' ').collect();
            if pars.len() == 3 {
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
                    (_, _) => {}
                }
            }
        } else if data.find("show_waiting_list ").is_some() {
            let pars: Vec<&str> = data.split(' ').collect();
            if pars.len() == 2 {
                match pars[1].parse::<i64>() {
                    Ok(event_id) => {
                        if self.config.public_lists || user.is_admin != false {
                            self.show_waiting_list(user, event_id, message);
                        } else {
                            warn!("not allowed");
                        }
                    }
                    Err(_e) => {}
                }
            }
        } else if data.find("show_presence_list ").is_some() {
            let pars: Vec<&str> = data.split(' ').collect();
            if pars.len() == 2 {
                match pars[1].parse::<i64>() {
                    Ok(event_id) => {
                        self.show_presence_list(event_id, user, message);
                    }
                    Err(_) => {}
                }
            }
        } else if data.find("confirm_presence ").is_some() {
            let pars: Vec<&str> = data.split(' ').collect();
            if pars.len() == 3 {
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
                    (_, _) => {}
                }
            }
        } else {
            self.api.spawn(user.id.text("Faied to parse query."));
            return false;
        }
        return true;
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
                        let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            format!(
                                "{} /{}({})/ {}",
                                format_ts(s.event.ts),
                                if s.state == 0 {
                                    s.event.max_adults - s.adults
                                } else {
                                    0
                                },
                                if s.state == 0 {
                                    s.event.max_children - s.children
                                } else {
                                    0
                                },
                                s.event.name
                            ),
                            format!("event {}", s.event.id),
                        ));
                        keyboard.add_row(v);
                    }
                    let text = "Программа\nвремя / взросл.(детск.) места  / мероприятие";
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
                        self.api.spawn(user_id.text(text).reply_markup(keyboard));
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
                let free_adults = s.event.max_adults - s.adults;
                let free_children = s.event.max_children - s.children;
                if s.state == 0 && s.my_adults < s.event.max_adults_per_reservation {
                    if free_adults > 0 {
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            if s.event.max_children != 0 {
                                "Записать взрослого +1"
                            } else {
                                "Записаться +1"
                            },
                            format!("sign_up {} adult nowait", s.event.id),
                        ));
                    } else {
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            if s.event.max_children != 0 {
                                "В лист ожидания взрослого +1"
                            } else {
                                "В лист ожидания +1"
                            },
                            format!("sign_up {} adult wait", s.event.id),
                        ));
                    }
                }
                if s.my_adults > 0 || s.my_wait_adults > 0 {
                    v.push(telegram_bot::types::InlineKeyboardButton::callback(
                        if s.event.max_children != 0 {
                            "Отписать взрослого -1"
                        } else {
                            "Отписаться -1"
                        },
                        format!("cancel {} adult", s.event.id),
                    ));
                }
                keyboard.add_row(v);
                let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                if s.state == 0 && s.my_children < s.event.max_children_per_reservation {
                    if free_children > 0 {
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            "Записать ребёнка +1",
                            format!("sign_up {} child nowait", s.event.id),
                        ));
                    } else {
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            "В лист ожидания ребёнка +1",
                            format!("sign_up {} child wait", s.event.id),
                        ));
                    }
                }
                if s.my_children > 0 || s.my_wait_children > 0 {
                    v.push(telegram_bot::types::InlineKeyboardButton::callback(
                        "Отписать ребёнка -1",
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
                                if s.event.max_children != 0 {
                                    list.push_str(&format!(" {} {}", p.adults, p.children));
                                } else {
                                    list.push_str(&format!(" {}", p.adults));
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
                        if s.state == 0 {
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
                    "\n \n<a href=\"{}\">{}</a>\nНачало: {}",
                    s.event.link,
                    s.event.name,
                    format_ts(s.event.ts)
                );
                if s.state == 0 {
                    if s.event.max_children != 0 {
                        text.push_str(&format!(
                            "\nВзрослые места: свободные - {}, моя бронь - {}",
                            free_adults, s.my_adults
                        ));
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
                    } else {
                        text.push_str(&format!(
                            "\nМеста: свободные - {}, моя бронь - {}",
                            free_adults, s.my_adults
                        ));
                        if s.my_wait_adults > 0 {
                            text.push_str(&format!(", лист ожидания - {}", s.my_wait_adults));
                        }
                    }
                } else {
                    text.push_str("\nЗапись остановлена.");
                }
                text.push_str(&format!("\n{}\n", list));
                if s.my_adults > 0
                    || s.my_wait_adults > 0
                    || s.my_children > 0
                    || s.my_wait_children > 0
                {
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
                    text.push_str("\nКоличество мест можно менять кнопками \"Записать/Отписать\". Примечание к брони можно добавить, послав сообщение боту.");
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
        match self.db.get_event(event_id, user.id.into()) {
            Ok(s) => {
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
