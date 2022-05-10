#[macro_use]
extern crate serde;
use chrono::DateTime;
use futures::StreamExt;
use std::collections::{HashMap, HashSet};
use std::{fs::File, io::prelude::*, time::Duration};
use telegram_bot::{Api, CanAnswerCallbackQuery, CanSendMessage, ToSourceChat, UpdateKind};
#[macro_use]
extern crate log;

pub mod db;
mod messages;
pub mod util;

use messages::*;
use util::*;

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Configuration {
    pub telegram_bot_token: String,
    pub admin_ids: String,
    pub admin_names: String,
    pub public_lists: bool,
    pub black_list_time: i64,
}

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

#[tokio::main]
async fn main() -> std::result::Result<(), String> {
    env_logger::init();
    let matches = clap::App::new("event-manager-telegram-bot")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("event-manager-telegram-bot")
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .help("Configuration file")
                .takes_value(true)
                .default_value(""),
        )
        .get_matches();

    let config = matches.value_of("config").unwrap();
    let mut f = File::open(config).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    let config = toml::from_str::<Configuration>(&contents)
        .map_err(|e| format!("Error loading configuration: {}", e.to_string()))
        .unwrap();

    let mut admin_ids: HashSet<i64> = HashSet::new();
    let ids: Vec<&str> = config.admin_ids.split(',').collect();
    for id in ids {
        if let Ok(v) = id.parse::<i64>() {
            admin_ids.insert(v);
        }
    }
    let mut admin_names: HashSet<String> = HashSet::new();
    let ids: Vec<&str> = config.admin_names.split(',').collect();
    for id in ids {
        if id.len() > 0 {
            admin_names.insert(id.to_string());
        }
    }

    let mut active_events: HashMap<i64, i64> = HashMap::new();

    let api = Api::new(config.telegram_bot_token.clone());
    let db = db::EventDB::open("./events.db3")
        .map_err(|e| format!("Failed to init db: {}", e.to_string()))
        .unwrap();
    let mut stream = api.stream();

    let mut timeout = tokio::time::Instant::now() + Duration::from_millis(6000);
    loop {
        match tokio::time::timeout_at(timeout, stream.next()).await {
            Ok(update) => {
                if let Some(update) = update {
                    let update = match update {
                        Ok(v) => v,
                        Err(e) => {
                            error!("Failed to parse update: {}", e.to_string());
                            continue;
                        }
                    };
                    if let UpdateKind::Message(msg) = update.kind {
                        debug!("message: {:?}", &msg);

                        if msg.from.is_bot {
                            warn!("Bot ignored");
                            continue;
                        }

                        let mut user_name1 = msg.from.first_name.clone();
                        if let Some(v) = msg.from.last_name.clone() {
                            user_name1.push_str(" ");
                            user_name1.push_str(&v);
                        }
                        let user_name2 = match msg.from.username.clone() {
                            Some(name) => name,
                            None => "".to_string(),
                        };

                        match &msg.kind {
                            telegram_bot::types::MessageKind::Text { data, .. } => {
                                debug!("Text: {}", data);
                                let is_admin = admin_ids.contains(&msg.from.id.into()) != false
                                    || admin_names.contains(&user_name2) != false;
                                // Direct link to subscribe
                                match data.find("/start ") {
                                    Some(v) => {
                                        if v == 0 {
                                            let pars: Vec<&str> = data.split(' ').collect();
                                            if pars.len() == 2 {
                                                match pars[1].parse::<i64>() {
                                                    Ok(event_id) => {
                                                        show_event(
                                                            &db,
                                                            &api,
                                                            msg.from.id,
                                                            event_id,
                                                            is_admin,
                                                            None,
                                                            config.public_lists,
                                                            None,
                                                        );
                                                        continue;
                                                    }
                                                    Err(_e) => {}
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }

                                if is_admin {
                                    // Broadcast message to a group?
                                    // /send confirmed <event> text
                                    // /send waiting <event> text
                                    match data.find("/send ") {
                                        Some(v) => {
                                            if v == 0 {
                                                let pars: Vec<&str> = data.splitn(4, ' ').collect();
                                                if pars.len() == 4 {
                                                    let waiting_list = match pars[1] {
                                                        "confirmed" => 0,
                                                        "waiting" => 1,
                                                        _ => 2,
                                                    };
                                                    if waiting_list < 2 {
                                                        match pars[2].parse::<i64>() {
                                                            Ok(event_id) => {
                                                                let text = format!(
                                                                    "<a href=\"tg://user?id={}\">{}</a>: {}",
                                                                    msg.from.id,
                                                                    user_name1,
                                                                    pars[3].to_string()
                                                                );
                                                                trace!("event id {}", event_id);
                                                                match db.get_participants(
                                                                    event_id,
                                                                    waiting_list,
                                                                ) {
                                                                    Ok(participants) => {
                                                                        api.spawn(
                                                                            msg.from
                                                                                .id
                                                                                .text(format!("The following message has been sent to {} participant(s):\n{}", participants.len(), text)).parse_mode(telegram_bot::types::ParseMode::Html),
                                                                        );
                                                                        for p in participants {
                                                                            api.spawn(
                                                                                telegram_bot::types::UserId::new(p.user_id)
                                                                                    .text(&text).parse_mode(telegram_bot::types::ParseMode::Html),
                                                                            );
                                                                            tokio::time::sleep(Duration::from_millis(40)).await; // not to hit the limits
                                                                        }
                                                                        continue;
                                                                    }
                                                                    Err(_) => {}
                                                                }
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }

                                    match data.find("/add_to_black_list ") {
                                        Some(v) => {
                                            if v == 0 {
                                                let pars: Vec<&str> = data.split(' ').collect();
                                                if pars.len() == 2 {
                                                    match pars[1].parse::<i64>() {
                                                        Ok(user_id) => {
                                                            if db.add_to_black_list(user_id).is_ok()
                                                                == false
                                                            {
                                                                error!("Failed to add user {} from black list", user_id);
                                                            }
                                                            show_black_list(&db, &api, msg.from.id);
                                                            continue;
                                                        }
                                                        Err(_e) => {}
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }

                                    match data.find("/remove_from_black_list ") {
                                        Some(v) => {
                                            if v == 0 {
                                                let pars: Vec<&str> = data.split(' ').collect();
                                                if pars.len() == 2 {
                                                    match pars[1].parse::<i64>() {
                                                        Ok(user_id) => {
                                                            if db
                                                                .remove_from_black_list(user_id)
                                                                .is_ok()
                                                                == false
                                                            {
                                                                error!("Failed to remove user {} from black list", user_id);
                                                            }
                                                            show_black_list(&db, &api, msg.from.id);
                                                            continue;
                                                        }
                                                        Err(_e) => {}
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }

                                    match data.find("/delete_event ") {
                                        Some(v) => {
                                            if v == 0 {
                                                let pars: Vec<&str> = data.split(' ').collect();
                                                if pars.len() == 2 {
                                                    match pars[1].parse::<i64>() {
                                                        Ok(event_id) => {
                                                            match db.delete_event(event_id) {
                                                                Ok(_) => {
                                                                    api.spawn(
                                                                        msg.from.id.text(format!(
                                                                            "Deleted"
                                                                        )),
                                                                    );
                                                                }
                                                                Err(e) => {
                                                                    api.spawn(msg.from.id.text(format!(
                                                                        "Failed to delete event: {}.",
                                                                        e
                                                                    )));
                                                                }
                                                            }
                                                            continue;
                                                        }
                                                        Err(_e) => {}
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }

                                    if data == "/show_black_list" {
                                        show_black_list(&db, &api, msg.from.id);
                                        continue;
                                    } else if data == "/help" {
                                        api.spawn(
                                            msg.to_source_chat()
                                                .text("Добавить мероприятие: \
                                                \n { \"name\":\"WIENXTRA CHILDREN'S ACTIVITIES for children up to 13 y.o.\", \"link\":\"https://t.me/storiesvienna/21\", \"start\":\"2022-05-29 15:00 +02:00\", \"remind\":\"2022-05-28 15:00 +02:00\", \"max_adults\":15, \"max_children\":15, \"max_adults_per_reservation\":15, \"max_children_per_reservation\":15 }\
                                                \n \nПослать сообщение: \
                                                \n /send confirmed <event> текст \
                                                \n /send waiting <event> текст \
                                                \n \nЧёрный список: \
                                                \n /add_to_black_list id \
                                                \n /remove_from_black_list id \
                                                \n /show_black_list \
                                                \n \n \
                                                \n /delete_event id \
                                                ").disable_preview(),
                                        );
                                        continue;
                                    }

                                    // Add new event?
                                    match data.find("{") {
                                        Some(_) => {
                                            let event: Result<NewEvent, serde_json::Error> =
                                                serde_json::from_str(&data);
                                            match event {
                                                Ok(v) => {
                                                    match (
                                                        DateTime::parse_from_str(
                                                            &v.start,
                                                            "%Y-%m-%d %H:%M  %z",
                                                        ),
                                                        DateTime::parse_from_str(
                                                            &v.remind,
                                                            "%Y-%m-%d %H:%M  %z",
                                                        ),
                                                    ) {
                                                        (Ok(ts), Ok(remind)) => {
                                                            match db.add_event(db::Event {
                                                                id: 0,
                                                                name: v.name,
                                                                link: v.link,
                                                                max_adults: v.max_adults,
                                                                max_children: v.max_children,
                                                                max_adults_per_reservation: v
                                                                    .max_adults_per_reservation,
                                                                max_children_per_reservation: v
                                                                    .max_children_per_reservation,
                                                                ts: ts.timestamp(),
                                                                remind: remind.timestamp(),
                                                            }) {
                                                                Ok(id) => {
                                                                    if id > 0 {
                                                                        api.spawn(
                                                                            msg.to_source_chat()
                                                                                .text(format!("Direct event link: https://t.me/sign_up_for_event_bot?start={}", id)),
                                                                        );
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    api.spawn(
                                                                        msg.to_source_chat().text(
                                                                            format!(
                                                                        "Failed to add event: {}.",
                                                                        e
                                                                    ),
                                                                        ),
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        (_, _) => {}
                                                    }
                                                }
                                                Err(e) => {
                                                    api.spawn(msg.to_source_chat().text(format!(
                                                        "Failed to parse json: {}.",
                                                        e
                                                    )));
                                                }
                                            }
                                            continue;
                                        }
                                        _ => {}
                                    }
                                }

                                if data == "/help" {
                                    api.spawn(
                                        msg.to_source_chat()
                                            .text("Этот бот поможет вам записываться на мероприятия: /start")
                                            .parse_mode(telegram_bot::types::ParseMode::Html),
                                    );
                                    continue;
                                } else if data == "/start" {
                                    show_event_list(&db, &api, msg.from.id, None);
                                    continue;
                                } else {
                                    let user_id: i64 = msg.from.id.into();
                                    let event_id = match active_events.get(&user_id) {
                                        Some(v) => *v,
                                        _ => match db.get_last_reservation_event(user_id) {
                                            Ok(v) => v,
                                            _ => 0,
                                        },
                                    };
                                    if event_id != 0 {
                                        match db.add_attachment(event_id, user_id, data) {
                                            Ok(_v) => {
                                                show_event(
                                                    &db,
                                                    &api,
                                                    msg.from.id,
                                                    event_id,
                                                    is_admin,
                                                    None,
                                                    config.public_lists,
                                                    match data.chars().any(char::is_numeric) {
                                                        true => Some("\n\nВНИМАНИЕ!\nВаше примечание содержит цифры. Они никак не влияют на количество забронированных мест. Количество мест можно менять только кнопками \"Записать/Отписать\".".to_string()),
                                                        false => None
                                                    }
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            _ => {
                                error!("Failed to parse message.");
                            }
                        }
                    } else if let UpdateKind::CallbackQuery(msg) = update.kind {
                        let mut user_name1 = msg.from.first_name.clone();
                        if let Some(v) = msg.from.last_name.clone() {
                            user_name1.push_str(" ");
                            user_name1.push_str(&v);
                        }
                        let user_name2 = match msg.from.username.clone() {
                            Some(name) => name,
                            None => "".to_string(),
                        };
                        let is_admin = admin_ids.contains(&msg.from.id.into()) != false
                            || admin_names.contains(&user_name2) != false;

                        debug!("callback: {:?}", &msg);
                        api.spawn(msg.acknowledge());
                        match msg.data {
                            Some(data) => {
                                if data == "event_list" {
                                    show_event_list(&db, &api, msg.from.id, msg.message);
                                    continue;
                                } else if data.find("event ").is_some() {
                                    let pars: Vec<&str> = data.split(' ').collect();
                                    if pars.len() == 2 {
                                        match pars[1].parse::<i64>() {
                                            Ok(event_id) => {
                                                active_events.insert(msg.from.id.into(), event_id);
                                                show_event(
                                                    &db,
                                                    &api,
                                                    msg.from.id,
                                                    event_id,
                                                    is_admin,
                                                    msg.message,
                                                    config.public_lists,
                                                    None,
                                                );
                                                continue;
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
                                                match db.sign_up(
                                                    event_id,
                                                    msg.from.id.into(),
                                                    &user_name1,
                                                    &user_name2,
                                                    is_adult as i64,
                                                    !is_adult as i64,
                                                    wait as i64,
                                                    get_unix_time(),
                                                ) {
                                                    Ok((_, black_listed)) => {
                                                        show_event(
                                                            &db,
                                                            &api,
                                                            msg.from.id,
                                                            event_id,
                                                            is_admin,
                                                            msg.message,
                                                            config.public_lists,
                                                            match black_listed {
                                                                true => { Some("\n\nВы добавлены в список ожидания.".to_string())},
                                                                false => None
                                                            }
                                                        );
                                                    }
                                                    Err(e) => {
                                                        api.spawn(msg.from.id.text(format!(
                                                            "Failed to add event: {}.",
                                                            e
                                                        )));
                                                    }
                                                }
                                                continue;
                                            }
                                            Err(_e) => {}
                                        }
                                    }
                                } else if data.find("cancel ").is_some() {
                                    let pars: Vec<&str> = data.split(' ').collect();
                                    if pars.len() == 3 {
                                        match pars[1].parse::<i64>() {
                                            Ok(event_id) => {
                                                let is_adult = pars[2] == "adult";
                                                match db.cancel(
                                                    event_id,
                                                    msg.from.id.into(),
                                                    is_adult as i64,
                                                ) {
                                                    Ok(update) => {
                                                        show_event(
                                                            &db,
                                                            &api,
                                                            msg.from.id,
                                                            event_id,
                                                            is_admin,
                                                            msg.message,
                                                            config.public_lists,
                                                            None,
                                                        );
                                                        notify_users_on_waiting_list(
                                                            &api, event_id, update,
                                                        );
                                                    }
                                                    Err(e) => {
                                                        api.spawn(msg.from.id.text(format!(
                                                            "Failed to add event: {}.",
                                                            e
                                                        )));
                                                    }
                                                }
                                                continue;
                                            }
                                            Err(_e) => {}
                                        }
                                    }
                                } else if data.find("wontgo ").is_some() {
                                    let pars: Vec<&str> = data.split(' ').collect();
                                    if pars.len() == 2 {
                                        match pars[1].parse::<i64>() {
                                            Ok(event_id) => {
                                                match db.wontgo(event_id, msg.from.id.into()) {
                                                    Ok(update) => {
                                                        api.spawn(
                                                            msg.from.id.text("Мы сожалеем, что вы не сможете пойти. Увидимся в другой раз. Спасибо!")
                                                        );
                                                        notify_users_on_waiting_list(
                                                            &api, event_id, update,
                                                        );
                                                    }
                                                    Err(e) => {
                                                        api.spawn(msg.from.id.text(format!(
                                                            "Failed to add event: {}.",
                                                            e
                                                        )));
                                                    }
                                                }
                                                continue;
                                            }
                                            Err(_e) => {}
                                        }
                                    }
                                } else if data.find("change_event_state ").is_some() {
                                    let pars: Vec<&str> = data.split(' ').collect();
                                    if pars.len() == 3 {
                                        match (pars[1].parse::<i64>(), pars[2].parse::<i64>()) {
                                            (Ok(event_id), Ok(state)) => {
                                                if is_admin != false {
                                                    match db.change_event_state(event_id, state) {
                                                        Ok(_) => {
                                                            show_event(
                                                                &db,
                                                                &api,
                                                                msg.from.id,
                                                                event_id,
                                                                is_admin,
                                                                None,
                                                                config.public_lists,
                                                                None,
                                                            );
                                                            continue;
                                                        }
                                                        Err(e) => {
                                                            api.spawn(msg.from.id.text(format!(
                                                                "Failed to close event: {}.",
                                                                e
                                                            )));
                                                        }
                                                    }
                                                } else {
                                                    warn!("not allowed");
                                                }
                                                continue;
                                            }
                                            (_, _) => {}
                                        }
                                    }
                                } else if data.find("show_waiting_list ").is_some() {
                                    let pars: Vec<&str> = data.split(' ').collect();
                                    if pars.len() == 2 {
                                        match pars[1].parse::<i64>() {
                                            Ok(event_id) => {
                                                if config.public_lists || is_admin != false {
                                                    show_waiting_list(
                                                        &db,
                                                        &api,
                                                        msg.from.id,
                                                        event_id,
                                                        msg.message,
                                                    );
                                                } else {
                                                    warn!("not allowed");
                                                }
                                                continue;
                                            }
                                            Err(_e) => {}
                                        }
                                    }
                                }
                                api.spawn(msg.from.id.text("Faied to find event."));
                            }
                            None => {}
                        }
                    }
                }
            }
            Err(_) => {
                // Timeout elapsed. Perform regular tasks.
                // Send out reminders if it is time.
                timeout = tokio::time::Instant::now() + Duration::from_millis(60000);
                let ts = get_unix_time();
                match db.get_user_reminders(ts) {
                    Ok(reminders) => {
                        for s in reminders {
                            let mut keyboard = telegram_bot::types::InlineKeyboardMarkup::new();
                            let mut v: Vec<telegram_bot::types::InlineKeyboardButton> = Vec::new();
                            v.push(telegram_bot::types::InlineKeyboardButton::callback(
                                "Отменить моё участие",
                                format!("wontgo {}", s.event_id),
                            ));
                            keyboard.add_row(v);
                            debug!("sending reminder");
                            api.spawn(
                                telegram_bot::types::UserId::new(s.user_id).text(
                                    format!("\nЗдравствуйте!\nНе забудьте, пожалуйста, что вы записались на\n<a href=\"{}\">{}</a>\
                                    \nНачало: {}\n\
                                    ВНИМАНИЕ! Если вы не сможете прийти и не отмените бронь заблаговременно, то в последующий месяц сможете бронировать только в листе ожидания.\n",
                                    s.link, s.name, format_ts(s.ts), )
                                )
                                .parse_mode(telegram_bot::types::ParseMode::Html)
                                .disable_preview()
                                .reply_markup(keyboard),
                            );
                            tokio::time::sleep(Duration::from_millis(40)).await; // not to hit the limits
                        }
                        if db.clear_user_reminders(ts).is_ok() == false {
                            error!("Failed to clear reminders at {}", ts);
                        }
                    }
                    Err(_e) => {
                        error!("Failed to get reminders at {}", ts);
                    }
                }
                // Clean up.
                if db.clear_old_events(ts).is_ok() == false {
                    error!("Failed to clear old events at {}", ts);
                }
                if db
                    .clear_black_list(ts - config.black_list_time * 24 * 60 * 60)
                    .is_ok()
                    == false
                {
                    error!("Failed to clear black list at {}", ts);
                }
            }
        }
    }
}
