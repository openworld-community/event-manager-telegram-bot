use crate::db::EventDB;
use crate::types::{Configuration, DialogState, EventState, User};
use crate::util::format_event_title;
use crate::{format_ts, get_unix_time};
use telegram_bot::{
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    Api, CanEditMessageText, CanSendMessage, MessageOrChannelPost,
};

/// User dialog handler.
pub struct MessageHandler<'a> {
    db: &'a EventDB,
    api: &'a Api,
    config: &'a Configuration,
}

impl<'a> MessageHandler<'a> {
    pub fn new(db: &'a EventDB, api: &'a Api, config: &'a Configuration) -> MessageHandler<'a> {
        MessageHandler { db, api, config }
    }

    /// Command line processor.
    pub fn process_message(&self, user: &User, data: &str, dialog_state: &mut DialogState) -> bool {
        let pars: Vec<&str> = data.splitn(3, ' ').collect();
        if pars.len() == 0 {
            return false;
        }
        match pars[0] {
            "/start" => {
                if pars.len() == 2 {
                    // Direct link
                    if let Ok(event_id) = pars[1].parse::<i64>() {
                        self.show_event(user, event_id, &None, None, 0);
                    }
                } else {
                    self.show_event_list(user.id, &None, 0);
                }
            }
            "/help" => {
                self.api.spawn(
                    user.id
                        .text(format!(
                            "Здесь вы можете бронировать места на мероприятия.\n \
                            \n /start - показать список мероприятий \
                            \n /help - эта подсказка \
                            \n <a href=\"{}\">Подробная инструкция</a>.",
                            self.config.help
                        ))
                        .disable_preview()
                        .parse_mode(telegram_bot::types::ParseMode::Html),
                );
            }
            _ => {
                // Message from user - try to add as attachment to the last reservation.
                self.add_attachment(&user, data, dialog_state);
            }
        }

        true
    }

    /// Callback query processor.
    pub fn process_query(
        &self,
        user: &User,
        data: &str,
        message: &Option<MessageOrChannelPost>,
        dialog_state: &mut DialogState,
    ) -> bool {
        let pars: Vec<&str> = data.splitn(4, ' ').collect();
        if pars.len() == 0 {
            return false;
        }
        match pars[0] {
            "event_list" if pars.len() == 2 => match pars[1].parse::<i64>() {
                Ok(offset) => {
                    self.show_event_list(user.id, message, offset);
                }
                _ => error!("Failed to parse query: {}", data),
            },
            "event" if pars.len() == 3 => match (pars[1].parse::<i64>(), pars[2].parse::<i64>()) {
                (Ok(event_id), Ok(offset)) => {
                    dialog_state.set_current_user_event(user.id.into(), event_id);
                    self.show_event(user, event_id, message, None, offset);
                }
                _ => error!("Failed to parse query: {}", data),
            },
            "sign_up" if pars.len() == 4 => match pars[1].parse::<i64>() {
                Ok(event_id) => {
                    let is_adult = pars[2] == "adult";
                    let wait = pars[3] == "wait";
                    match self.db.sign_up(
                        event_id,
                        user,
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
                                    Some(format!("\n\nИзвините, но бронирование невозможно, поскольку ранее Вы не использовали и не отменили бронь. \
                                        Если это ошибка, пожалуйста, свяжитесь с <a href=\"tg://user?id={}\">поддержкой</a> и сообщите код {}. <a href=\"{}\">Инструкция</a>.", self.config.support, user.id, self.config.help))
                                } else {
                                    None
                                },
                                0,
                            );
                        }
                        Err(e) => {
                            self.api.spawn(user.id.text(format!("{}", e)));
                        }
                    }
                }
                Err(e) => {
                    error!("Failed sign up: {}", e);
                }
            },
            "cancel" if pars.len() == 3 => {
                if let Ok(event_id) = pars[1].parse::<i64>() {
                    let is_adult = pars[2] == "adult";
                    match self.db.cancel(event_id, user.id.into(), is_adult as i64) {
                        Ok(_) => {
                            let mut ps = None;
                            if self.is_too_late_to_cancel(event_id, user) {
                                if let Ok(s) = self.db.get_event(event_id, user.id.into()) {
                                    if s.adults.my_reservation + s.children.my_reservation == 0 {
                                        // Complete cancellation
                                        if self
                                            .db
                                            .ban_user(
                                                user.id.into(),
                                                &user.user_name1,
                                                &user.user_name2,
                                                &format!(
                                                    "late cancel {} {}",
                                                    format_ts(s.event.ts),
                                                    s.event.name
                                                ),
                                                self.config.cancel_future_reservations_on_ban,
                                            )
                                            .is_ok()
                                            == false
                                        {
                                            error!("Failed to add user {} to black list", user.id);
                                        }
                                        ps = Some(format!("\n\nВНИМАНИЕ!\nК сожалению, вы отказались от билетов слишком поздно и не сможете больше бронировать бесплатные билеты."));
                                    }
                                }
                            }
                            self.show_event(user, event_id, message, ps, 0);
                        }
                        Err(e) => {
                            self.api.spawn(
                                user.id
                                    .text(format!("Failed to cancel reservation: {}.", e)),
                            );
                        }
                    }
                }
            }
            "wontgo" if pars.len() == 2 => match pars[1].parse::<i64>() {
                Ok(event_id) => match self.db.wontgo(event_id, user.id.into()) {
                    Ok(_) => {
                        if self.is_too_late_to_cancel(event_id, user) {
                            self.api
                                .spawn(user.id.text("К сожалению, вы отказываетесь от билетов слишком поздно и не сможете больше бронировать бесплатные билеты."));
                        } else {
                            self.api.spawn(user.id.text("Мы сожалеем, что вы не сможете пойти. Увидимся в другой раз. Спасибо!"));
                        }
                    }
                    Err(e) => {
                        self.api
                            .spawn(user.id.text(format!("Failed to add event: {}.", e)));
                    }
                },
                Err(e) => {
                    error!("Failed to cancel: {}", e);
                }
            },
            "change_event_state" if pars.len() == 3 => {
                match (pars[1].parse::<i64>(), pars[2].parse::<i64>()) {
                    (Ok(event_id), Ok(state)) => {
                        if user.is_admin != false {
                            match self.db.change_event_state(event_id, state) {
                                Ok(_) => {
                                    self.show_event(user, event_id, &None, None, 0);
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
                    _ => error!("Failed to parse query: {}", data),
                }
            }
            "show_waiting_list" if pars.len() == 3 => {
                match (pars[1].parse::<i64>(), pars[2].parse::<i64>()) {
                    (Ok(event_id), Ok(offset)) => {
                        if self.config.public_lists || user.is_admin != false {
                            self.show_waiting_list(user, event_id, message, offset);
                        } else {
                            warn!("not allowed");
                        }
                    }
                    _ => error!("Failed to parse query: {}", data),
                }
            }
            "show_presence_list" if pars.len() == 3 => {
                match (pars[1].parse::<i64>(), pars[2].parse::<i64>()) {
                    (Ok(event_id), Ok(offset)) => {
                        self.show_presence_list(event_id, user, message, offset);
                    }
                    _ => error!("Failed to parse query: {}", data),
                }
            }
            "confirm_presence" if pars.len() == 4 => {
                match (
                    pars[1].parse::<i64>(),
                    pars[2].parse::<i64>(),
                    pars[3].parse::<i64>(),
                ) {
                    (Ok(event_id), Ok(user_id), Ok(offset)) => {
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
                                    self.show_presence_list(event_id, user, message, offset);
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
                    _ => error!("Failed to parse query: {}", data),
                }
            }
            _ => {
                self.api.spawn(user.id.text("Failed to parse query."));
                return false;
            }
        }

        true
    }

    pub fn add_attachment(&self, user: &User, data: &str, dialog_state: &mut DialogState) {
        let user_id: i64 = user.id.into();
        let event_id = match dialog_state.get_current_user_event(&user.id) {
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
                        0,
                    );
                }
                _ => error!("Failed to parse attachment: {}", data),
            }
        }
    }

    pub fn show_event_list(
        &self,
        user_id: telegram_bot::UserId,
        callback: &Option<MessageOrChannelPost>,
        offset: i64,
    ) {
        match self
            .db
            .get_events(user_id.into(), offset, self.config.event_list_page_size)
        {
            Ok(events) => {
                let mut keyboard = InlineKeyboardMarkup::new();
                events
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
                                        (s.event.max_adults - s.adults.reserved
                                            + s.event.max_children
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
                    .for_each(|r| {
                        keyboard.add_row(r);
                    });

                let header_text = if offset != 0 || events.len() != 0 {
                    format!("Программа\nвремя / взросл.(детск.) места  / мероприятие\n<a href=\"{}\">инструкция</a>", self.config.help)
                } else {
                    "Нет мероприятий.".to_string()
                };

                add_pagination(
                    &mut keyboard,
                    "event_list",
                    events.len() as i64,
                    self.config.event_list_page_size,
                    offset,
                );

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
                    self.api.spawn(
                        user_id
                            .text(header_text)
                            .parse_mode(telegram_bot::types::ParseMode::Html)
                            .disable_preview()
                            .reply_markup(keyboard),
                    );
                }
            }
            Err(e) => {
                self.api
                    .spawn(user_id.text(format!("Failed to query events: {}", e)));
            }
        }
    }

    pub fn show_event(
        &self,
        user: &User,
        event_id: i64,
        callback: &Option<MessageOrChannelPost>,
        ps: Option<String>,
        offset: i64,
    ) {
        match self.db.get_event(event_id, user.id.into()) {
            Ok(s) => {
                let mut keyboard = InlineKeyboardMarkup::new();
                let mut v: Vec<InlineKeyboardButton> = Vec::new();
                let free_adults = s.event.max_adults - s.adults.reserved;
                let free_children = s.event.max_children - s.children.reserved;
                let no_age_distinction = s.event.max_adults == 0 || s.event.max_children == 0;
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
                keyboard.add_row(v);
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
                keyboard.add_row(v);
                let mut v: Vec<InlineKeyboardButton> = Vec::new();
                v.push(InlineKeyboardButton::callback(
                    "Список мероприятий",
                    "event_list 0",
                ));
                let mut list = "".to_string();
                let mut participants_len: i64 = 0;
                if self.config.public_lists || user.is_admin {
                    match self
                        .db
                        .get_participants(event_id, 0, offset, self.config.event_page_size)
                    {
                        Ok(participants) => {
                            participants_len = participants.len() as i64;
                            if user.is_admin {
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
                                        let id = if user.is_admin {
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
                                            entry
                                                .push_str(&format!(" {} {}", p.adults, p.children));
                                        }
                                        if let Some(a) = &p.attachment {
                                            entry.push_str(&format!(" {}", a));
                                        }
                                        entry.push_str("\n");
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
                    if user.is_admin {
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
                        if let Ok(check) = self.db.is_group_leader(event_id, user.id.into()) {
                            if check {
                                v.push(InlineKeyboardButton::callback(
                                    "Присутствие",
                                    format!("show_presence_list {} 0", event_id),
                                ));
                            }
                        }
                    }
                }
                keyboard.add_row(v);
                add_pagination(
                    &mut keyboard,
                    &format!("event {}", event_id),
                    participants_len,
                    self.config.event_page_size,
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
                if user.is_admin
                    || s.adults.my_reservation > 0
                    || s.adults.my_waiting > 0
                    || s.children.my_reservation > 0
                    || s.children.my_waiting > 0
                {
                    if let Ok(messages) = self.db.get_messages(
                        event_id,
                        if user.is_admin {
                            None
                        } else {
                            Some(
                                (s.adults.my_reservation == 0 && s.children.my_reservation == 0)
                                    as i64,
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

                    if self.config.public_lists == false {
                        match self.db.get_attachment(event_id, user.id.into()) {
                            Ok(v) => {
                                if let Some(attachment) = v {
                                    text.push_str(&format!("\nПримечание: {}.", attachment));
                                }
                            }
                            Err(e) => error!("Failed to get attachment: {}", e),
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
            Err(e) => {
                error!("Failed to fetch event: {}", e);
            }
        }
    }
    fn show_waiting_list(
        &self,
        user: &User,
        event_id: i64,
        callback: &Option<MessageOrChannelPost>,
        offset: i64,
    ) {
        let mut list = "".to_string();
        let no_age_distinction;
        match self.db.get_event(event_id, user.id.into()) {
            Ok(s) => {
                no_age_distinction = s.event.max_adults == 0 || s.event.max_children == 0;
                list.push_str(&format!(
                    "\n \n{}\nНачало: {}\n",
                    format_event_title(&s.event),
                    format_ts(s.event.ts)
                ));
            }
            Err(e) => {
                self.api.spawn(user.id.text("Failed to find event"));
                error!("Failed to find event: {}", e);
                return;
            }
        }
        match self
            .db
            .get_participants(event_id, 1, offset, self.config.event_page_size)
        {
            Ok(participants) => {
                let mut keyboard = InlineKeyboardMarkup::new();
                add_pagination(
                    &mut keyboard,
                    &format!("show_waiting_list {}", event_id),
                    participants.len() as i64,
                    self.config.event_page_size,
                    offset,
                );

                let mut v: Vec<InlineKeyboardButton> = Vec::new();
                v.push(InlineKeyboardButton::callback(
                    "Назад",
                    format!("event {} 0", event_id),
                ));
                keyboard.add_row(v);

                if participants.len() == 0 {
                    list.push_str("Пустой список ожидания.");
                } else {
                    list.push_str("Список ожидания:");
                    list.push_str(
                        &participants
                            .iter()
                            .map(|p| {
                                let mut entry = "".to_string();
                                let id = if user.is_admin {
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
            Err(e) => {
                error!("Failed to get participants: {}", e);
            }
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
        offset: i64,
    ) {
        let mut header = "".to_string();
        match self.db.get_event(event_id, user.id.into()) {
            Ok(s) => {
                header.push_str(&format!(
                    "\n \n{}\nНачало: {}\n",
                    format_event_title(&s.event),
                    format_ts(s.event.ts)
                ));
            }
            Err(e) => {
                self.api.spawn(user.id.text("Failed to find event"));
                error!("Failed to find event: {}", e);
                return;
            }
        }
        match self
            .db
            .get_presence_list(event_id, offset, self.config.presence_page_size)
        {
            Ok(participants) => {
                let mut keyboard = InlineKeyboardMarkup::new();
                if participants.len() != 0 {
                    header.push_str("Пожалуйста, выберите присутствующих:");
                }

                participants
                    .iter()
                    .map(|p| {
                        vec![{
                            let mut text;
                            if p.user_name2.len() > 0 {
                                text =
                                    format!("{} ({}) {}", p.user_name1, p.user_name2, p.reserved);
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
                    .for_each(|r| {
                        keyboard.add_row(r);
                    });

                add_pagination(
                    &mut keyboard,
                    &format!("show_presence_list {}", event_id),
                    participants.len() as i64,
                    self.config.presence_page_size,
                    offset,
                );

                let mut v: Vec<InlineKeyboardButton> = Vec::new();
                v.push(InlineKeyboardButton::callback(
                    "Назад",
                    format!("event {} 0", event_id),
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
            Err(e) => {
                error!("Failed to get precense list: {}", e);
            }
        }
    }
}

pub fn add_pagination(
    keyboard: &mut InlineKeyboardMarkup,
    cmd: &str,
    participants: i64,
    limit: i64,
    offset: i64,
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
        keyboard.add_row(pagination);
    }
}
