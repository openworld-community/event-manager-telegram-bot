use crate::db::EventDB;
use crate::message_handler::MessageHandler;
use crate::types::{Configuration, Event, MessageType, User};
use crate::util::{format_event_title, format_ts};
use chrono::DateTime;
use std::collections::HashSet;
use telegram_bot::{Api, CanEditMessageText, CanSendMessage, MessageOrChannelPost};

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
struct NewEvent {
    id: Option<i64>,
    name: String,
    link: String,
    start: String,
    remind: String,
    max_adults: i64,
    max_children: i64,
    max_adults_per_reservation: i64,
    max_children_per_reservation: i64,
}

/// Admin dialog handler.
pub struct AdminMessageHandler<'a> {
    db: &'a EventDB,
    api: &'a Api,
    config: &'a Configuration,
    message_handler: &'a MessageHandler<'a>,
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

    /// Command line processor.
    pub fn process_message(&self, user: &User, data: &str, admins: &HashSet<i64>) -> bool {
        let pars: Vec<&str> = data.splitn(4, ' ').collect();
        if pars.len() == 0 {
            return false;
        }
        let mut response = "".to_string();
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
                    if let Ok(event_id) = pars[2].parse::<i64>() {
                        match self.db.get_event(event_id, user.id.into()) {
                            Ok(s) => {
                                let text = format!(
                                        "<a href=\"tg://user?id={}\">{}</a>:\nСообщение по мероприятию {} (Начало: {})\n{}",
                                        user.id,
                                        user.user_name1,
                                        format_event_title(&s.event),
                                        format_ts(s.event.ts),
                                        pars[3].to_string()
                                    );

                                if self
                                    .db
                                    .enqueue_message(
                                        event_id,
                                        &user.user_name1,
                                        waiting_list,
                                        MessageType::Direct,
                                        &text,
                                        crate::util::get_unix_time(),
                                    )
                                    .is_ok()
                                {
                                    self.api.spawn(
                                        user.id
                                                .text(format!("The following message has been scheduled for sending:\n{}", text)).parse_mode(telegram_bot::types::ParseMode::Html).disable_preview(),
                                        );
                                } else {
                                    self.api.spawn(
                                        user.id
                                            .text(format!("Failed to send message."))
                                            .parse_mode(telegram_bot::types::ParseMode::Html)
                                            .disable_preview(),
                                    );
                                }
                            }
                            Err(e) => {
                                self.api.spawn(user.id.text("Failed to find event"));
                                error!("Failed to find event: {}", e);
                            }
                        }
                    }
                }
            }
            "/ban" if pars.len() == 2 => {
                if let Ok(user_id) = pars[1].parse::<i64>() {
                    if self
                        .db
                        .add_to_black_list(user_id, self.config.cancel_future_reservations_on_ban)
                        .is_ok()
                        == false
                    {
                        error!("Failed to add user {} to black list", user_id);
                    }
                    self.show_black_list(user, 0, &None);
                }
            }
            "/remove_from_black_list" if pars.len() == 2 => {
                if let Ok(user_id) = pars[1].parse::<i64>() {
                    if self.db.remove_from_black_list(user_id).is_ok() == false {
                        error!("Failed to remove user {} from black list", user_id);
                    }
                    self.show_black_list(user, 0, &None);
                }
            }
            "/delete_event" if pars.len() == 2 => {
                if let Ok(event_id) = pars[1].parse::<i64>() {
                    if self.config.automatic_blacklisting {
                        if let Err(e) = self.db.blacklist_absent_participants(
                            event_id,
                            admins,
                            self.config.cancel_future_reservations_on_ban,
                        ) {
                            self.api.spawn(
                                user.id.text(format!(
                                    "Failed to blacklist absent participants: {}.",
                                    e
                                )),
                            );
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
            "/delete_reservation" if pars.len() == 3 => {
                if let (Ok(event_id), Ok(user_id)) =
                    (pars[1].parse::<i64>(), pars[2].parse::<i64>())
                {
                    response = match self.db.delete_reservation(event_id, user_id) {
                        Ok(_) => {
                            format!("Reservation deleted.")
                        }
                        Err(e) => {
                            format!("Failed to delete reservation: {}.", e)
                        }
                    };
                }
            }
            "/set_group_leader" if pars.len() == 3 => {
                if let (Ok(event_id), Ok(user_id)) =
                    (pars[1].parse::<i64>(), pars[2].parse::<i64>())
                {
                    response = self.db.set_group_leader(event_id, user_id).map_or_else(
                        |e| format!("Failed to set group leader: {}.", e),
                        |_| "Group leader set.".to_string(),
                    );
                }
            }
            "/show_black_list" => {
                self.show_black_list(user, 0, &None);
            }
            "/set_event_limits" if pars.len() == 4 => {
                if let (Ok(event_id), Ok(max_adults), Ok(max_children)) = (
                    pars[1].parse::<i64>(),
                    pars[2].parse::<i64>(),
                    pars[3].parse::<i64>(),
                ) {
                    response = self
                        .db
                        .set_event_limits(event_id, max_adults, max_children)
                        .map_or_else(
                            |e| format!("Failed to set event limits: {}.", e),
                            |_| "Event limits updated.".to_string(),
                        );
                }
            }
            "/help" => {
                self.api.spawn(
                        user.id
                            .text("Добавить мероприятие: \
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
                            ").disable_preview(),
                    );
            }
            _ => {
                if let Some(ch) = data.chars().next() {
                    if ch == '{' {
                        self.add_event(user, data);
                        return true;
                    }
                }
                return false;
            }
        }

        if response.len() > 0 {
            self.api.spawn(user.id.text(response));
        }
        true
    }

    /// Callback query processor.
    pub fn process_query(
        &self,
        user: &User,
        data: &str,
        message: &Option<MessageOrChannelPost>,
    ) -> bool {
        let pars: Vec<&str> = data.splitn(3, ' ').collect();
        if pars.len() == 0 {
            return false;
        }
        match pars[0] {
            "change_event_state" if pars.len() == 3 => {
                match (pars[1].parse::<i64>(), pars[2].parse::<i64>()) {
                    (Ok(event_id), Ok(state)) => {
                        match self.db.change_event_state(event_id, state) {
                            Ok(_) => {
                                self.message_handler
                                    .show_event(user, event_id, message, None, 0);
                            }
                            Err(e) => {
                                self.api
                                    .spawn(user.id.text(format!("Failed to close event: {}.", e)));
                            }
                        }
                    }
                    _ => error!("Failed to parse command: {}", data),
                }
            }
            "confirm_remove_from_black_list" if pars.len() == 2 => {
                if let Ok(user_id) = pars[1].parse::<i64>() {
                    if let Ok(reason) = self.db.get_ban_reason(user_id) {
                        let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                        let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            "да",
                            format!("remove_from_black_list {}", user_id),
                        ));
                        v.push(telegram_bot::types::InlineKeyboardButton::callback(
                            "нет",
                            "show_black_list 0",
                        ));
                        keyboard.add_row(v);

                        if let Some(msg) = message {
                            let header = format!("Причина бана: {reason}\nУдалить пользавателя <a href=\"tg://user?id={0}\">{0}</a> из чёрного списка?", user_id);
                            self.api.spawn(
                                msg.edit_text(header)
                                    .parse_mode(telegram_bot::types::ParseMode::Html)
                                    .reply_markup(keyboard),
                            );
                        }
                    }
                }
            }
            "remove_from_black_list" if pars.len() == 2 => {
                if let Ok(user_id) = pars[1].parse::<i64>() {
                    if self.db.remove_from_black_list(user_id).is_ok() == false {
                        error!("Failed to remove user {} from black list", user_id);
                    }
                    self.show_black_list(user, 0, message);
                }
            }
            "show_black_list" if pars.len() == 2 => {
                if let Ok(offset) = pars[1].parse::<i64>() {
                    self.show_black_list(user, offset, message);
                }
            }
            _ => {
                return false;
            }
        }

        true
    }

    pub fn add_event(&self, user: &User, data: &str) {
        match serde_json::from_str::<NewEvent>(&data) {
            Ok(v) => {
                match (
                    DateTime::parse_from_str(&v.start, "%Y-%m-%d %H:%M  %z"),
                    DateTime::parse_from_str(&v.remind, "%Y-%m-%d %H:%M  %z"),
                ) {
                    (Ok(ts), Ok(remind)) => {
                        match self.db.add_event(Event {
                            id: v.id.unwrap_or(0),
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
                    _ => {
                        self.api
                            .spawn(user.id.text("Failed to parse date.".to_string()));
                    }
                }
            }
            Err(e) => {
                self.api
                    .spawn(user.id.text(format!("Failed to parse json: {}.", e)));
            }
        }
    }

    fn show_black_list(&self, user: &User, offset: i64, callback: &Option<MessageOrChannelPost>) {
        match self
            .db
            .get_black_list(offset, self.config.presence_page_size)
        {
            Ok(participants) => {
                let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                participants
                    .iter()
                    .map(|u| {
                        vec![telegram_bot::types::InlineKeyboardButton::callback(
                            if u.user_name2.len() > 0 {
                                format!("{} ({}) {}", u.user_name1, u.user_name2, u.id)
                            } else {
                                format!("{} {}", u.user_name1, u.id)
                            },
                            format!("confirm_remove_from_black_list {}", u.id),
                        )]
                    })
                    .for_each(|r| {
                        keyboard.add_row(r);
                    });

                crate::message_handler::add_pagination(
                    &mut keyboard,
                    "show_black_list",
                    participants.len() as i64,
                    self.config.presence_page_size,
                    offset,
                );

                let header = if participants.len() != 0 || offset > 0 {
                    "Чёрный список. Нажмите кнопку чтобы удалить из списка."
                } else {
                    "Чёрный список пуст."
                };
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
            Err(e) => {
                error!("Failed to get black list: {}", e);
            }
        }
    }
}
