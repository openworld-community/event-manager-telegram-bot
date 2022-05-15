use crate::db::{Event, EventDB};
use crate::message_handler::{Configuration, MessageHandler, User};
use crate::util::*;
use chrono::DateTime;
use std::collections::HashMap;
use telegram_bot::{Api, CanEditMessageText, CanSendMessage, MessageOrChannelPost};

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct NewEvent {
    pub name: String,
    pub link: String,
    pub start: String,
    pub remind: String,
    pub max_adults: i64,
    pub max_children: i64,
    pub max_adults_per_reservation: i64,
    pub max_children_per_reservation: i64,
}

pub struct AdminMessageHandler<'a> {
    pub db: &'a EventDB,
    pub api: &'a Api,
    pub config: &'a Configuration,
    pub message_handler: &'a MessageHandler<'a>,
}

impl<'a> AdminMessageHandler<'a> {
    pub fn new(
        db: &'a EventDB,
        api: &'a Api,
        config: &'a Configuration,
        message_handler: &'a MessageHandler,
    ) -> AdminMessageHandler<'a> {
        AdminMessageHandler {
            db,
            api,
            config,
            message_handler,
        }
    }

    pub async fn process_message(
        &self,
        user: &User,
        data: &str,
    ) -> bool {
        if let Some(v) = data.find("/send ") {
            // Broadcast message to a group?
            // /send confirmed <event> text
            // /send waiting <event> text
            if v == 0 {
                let pars: Vec<&str> = data.splitn(4, ' ').collect();
                if pars.len() == 4 {
                    let waiting_list = match pars[1] {
                        "confirmed" => 0,
                        "waiting" => 1,
                        _ => 2,
                    };
                    if waiting_list < 2 {
                        if let Ok(event_id) = pars[2].parse::<i64>() {
                            match self.db.get_event(event_id, user.id.into()) {
                                Ok(s) => {
                                    let text = format!(
                                        "<a href=\"tg://user?id={}\">{}</a>:\nСообщение по мероприятию <a href=\"{}\">{}</a> (Начало: {})\n{}",
                                        user.id,
                                        user.user_name1,
                                        s.event.link,
                                        s.event.name,
                                        format_ts(s.event.ts),
                                        pars[3].to_string()
                                    );
                                    trace!("event id {}", event_id);
                                    if let Ok(participants) =
                                        self.db.get_participants(event_id, waiting_list)
                                    {
                                        self.api.spawn(
                                            user.id
                                                .text(format!("The following message has been sent to {} participant(s):\n{}", participants.len(), text)).parse_mode(telegram_bot::types::ParseMode::Html).disable_preview(),
                                        );
                                        for p in &participants {
                                            self.api.spawn(
                                                telegram_bot::types::UserId::new(p.user_id)
                                                    .text(&text)
                                                    .parse_mode(
                                                        telegram_bot::types::ParseMode::Html,
                                                    ),
                                            );
                                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                                40,
                                            ))
                                            .await;
                                            // not to hit the limits
                                        }
                                    }
                                }
                                Err(_e) => {
                                    self.api.spawn(user.id.text("Failed to find event"));
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(v) = data.find("/add_to_black_list ") {
            // /add_to_black_list id
            if v == 0 {
                let pars: Vec<&str> = data.split(' ').collect();
                if pars.len() == 2 {
                    if let Ok(user_id) = pars[1].parse::<i64>() {
                        if self.db.add_to_black_list(user_id).is_ok() == false {
                            error!("Failed to add user {} from black list", user_id);
                        }
                        self.show_black_list(user, &None);
                    }
                }
            }
        } else if let Some(v) = data.find("/remove_from_black_list ") {
            // /remove_from_black_list id
            if v == 0 {
                let pars: Vec<&str> = data.split(' ').collect();
                if pars.len() == 2 {
                    if let Ok(user_id) = pars[1].parse::<i64>() {
                        if self.db.remove_from_black_list(user_id).is_ok() == false {
                            error!("Failed to remove user {} from black list", user_id);
                        }
                        self.show_black_list(user, &None);
                    }
                }
            }
        } else if let Some(v) = data.find("/delete_event ") {
            // /delete_event id
            if v == 0 {
                let pars: Vec<&str> = data.split(' ').collect();
                if pars.len() == 2 {
                    if let Ok(event_id) = pars[1].parse::<i64>() {
                        if self.config.automatic_blacklisting {
                            if let Err(e) = self.db.blacklist_absent_participants(event_id) {
                                self.api.spawn(user.id.text(format!(
                                    "Failed to blacklist absent participants: {}.",
                                    e
                                )));
                            }
                        }
                        match self.db.delete_event(event_id) {
                            Ok(_) => {
                                self.api.spawn(user.id.text("Deleted"));
                            }
                            Err(e) => {
                                self.api
                                    .spawn(user.id.text(format!("Failed to delete event: {}.", e)));
                            }
                        }
                    }
                }
            }
        } else if let Some(v) = data.find("/set_group_leader ") {
            if v == 0 {
                let pars: Vec<&str> = data.split(' ').collect();
                if pars.len() == 3 {
                    match (pars[1].parse::<i64>(), pars[2].parse::<i64>()) {
                        (Ok(event_id), Ok(user_id)) => {
                            match self.db.set_group_leader(event_id, user_id) {
                                Ok(_) => {
                                    self.api.spawn(user.id.text(format!("Group leader set.")));
                                }
                                Err(e) => {
                                    self.api.spawn(
                                        user.id.text(format!("Failed to confirm presence: {}.", e)),
                                    );
                                }
                            }
                        }
                        (_, _) => {}
                    }
                }
            }
        } else if data == "/show_black_list" {
            self.show_black_list(user, &None);
        } else if data == "/help" {
            self.api.spawn(
                user.id
                    .text("Добавить мероприятие: \
                    \n { \"name\":\"WIENXTRA CHILDREN'S ACTIVITIES for children up to 13 y.o.\", \"link\":\"https://t.me/storiesvienna/21\", \"start\":\"2022-05-29 15:00 +02:00\", \"remind\":\"2022-05-28 15:00 +02:00\", \"max_adults\":15, \"max_children\":15, \"max_adults_per_reservation\":15, \"max_children_per_reservation\":15 }\
                    \n \nПослать сообщение: \
                    \n /send confirmed <event> текст \
                    \n /send waiting <event> текст \
                    \n \nЧёрный список: \
                    \n /add_to_black_list <user> \
                    \n /remove_from_black_list <user> \
                    \n /show_black_list \
                    \n \n \
                    \n /delete_event <event> \
                    \n /set_group_leader <event> <user> \
                    ").disable_preview(),
            );
        } else {
            if let Some(ch) = data.chars().next() {
                if ch == '{' {
                    self.add_event(user, data);
                    return true;
                }
            }
            return false;
        }
        true
    }

    pub fn process_query(
        &self,
        user: &User,
        data: &str,
        message: &Option<MessageOrChannelPost>,
        _active_events: &mut HashMap<i64, i64>,
    ) -> bool {
        if data.find("change_event_state ").is_some() {
            let pars: Vec<&str> = data.split(' ').collect();
            if pars.len() == 3 {
                match (pars[1].parse::<i64>(), pars[2].parse::<i64>()) {
                    (Ok(event_id), Ok(state)) => {
                        match self.db.change_event_state(event_id, state) {
                            Ok(_) => {
                                self.message_handler
                                    .show_event(user, event_id, message, None);
                            }
                            Err(e) => {
                                self.api
                                    .spawn(user.id.text(format!("Failed to close event: {}.", e)));
                            }
                        }
                    }
                    (_, _) => {}
                }
            }
        } else if let Some(v) = data.find("remove_from_black_list ") {
            if v == 0 {
                let pars: Vec<&str> = data.split(' ').collect();
                if pars.len() == 2 {
                    if let Ok(user_id) = pars[1].parse::<i64>() {
                        if self.db.remove_from_black_list(user_id).is_ok() == false {
                            error!("Failed to remove user {} from black list", user_id);
                        }
                        self.show_black_list(user, message);
                    }
                }
            }
        } else {
            return false;
        }
        true
    }

    pub fn add_event(&self, user: &User, data: &str) {
        let event: Result<NewEvent, serde_json::Error> = serde_json::from_str(&data);
        match event {
            Ok(v) => {
                match (
                    DateTime::parse_from_str(&v.start, "%Y-%m-%d %H:%M  %z"),
                    DateTime::parse_from_str(&v.remind, "%Y-%m-%d %H:%M  %z"),
                ) {
                    (Ok(ts), Ok(remind)) => {
                        match self.db.add_event(Event {
                            id: 0,
                            name: v.name,
                            link: v.link,
                            max_adults: v.max_adults,
                            max_children: v.max_children,
                            max_adults_per_reservation: v.max_adults_per_reservation,
                            max_children_per_reservation: v.max_children_per_reservation,
                            ts: ts.timestamp(),
                            remind: remind.timestamp(),
                        }) {
                            Ok(id) => {
                                if id > 0 {
                                    self.api.spawn(
                                        user.id
                                            .text(format!("Direct event link: https://t.me/sign_up_for_event_bot?start={}", id)),
                                    );
                                }
                            }
                            Err(e) => {
                                self.api
                                    .spawn(user.id.text(format!("Failed to add event: {}.", e)));
                            }
                        }
                    }
                    (_, _) => {}
                }
            }
            Err(e) => {
                self.api
                    .spawn(user.id.text(format!("Failed to parse json: {}.", e)));
            }
        }
    }

    fn show_black_list(&self, user: &User, callback: &Option<MessageOrChannelPost>) {
        match self.db.get_black_list() {
            Ok(participants) => {
                let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                if participants.len() != 0 {
                    for u in &participants {
                        let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                        let text;
                        if u.user_name2.len() > 0 {
                            text = format!("{} ({}) {}", u.user_name1, u.user_name2, u.id);
                        } else {
                            text = format!("{} {}", u.user_name1, u.id);
                        }
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            text,
                            format!("remove_from_black_list {}", u.id),
                        ));
                        keyboard.add_row(v);
                    }

                    let header = "Чёрный список. Нажмите кнопку чтобы удалить из списка.";
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
                } else {
                    self.api.spawn(user.id.text("Чёрный список пуст."));
                }
            }
            Err(_e) => {}
        }
    }
}
