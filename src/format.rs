use crate::types::{Connection, Event};
use crate::types::{EventState, Participant};
use chrono::{DateTime, NaiveDateTime, Utc};

use crate::db;
use db::EventStats;

pub fn from_timestamp(ts: u64) -> DateTime<Utc> {
    let naive =
        NaiveDateTime::from_timestamp_opt(ts as i64, 0).expect("NaiveDateTime Unwrap Error");
    let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
    datetime
}

pub fn ts(ts: u64) -> String {
    let datetime = from_timestamp(ts);
    datetime.format("%d.%m %H:%M").to_string()
}

pub fn event_title(event: &Event) -> String {
    if event.link.len() > 0 {
        format!("<a href=\"{}\">{}</a>", event.link, event.name,)
    } else {
        event.name.to_string()
    }
}

pub fn header(
    s: &EventStats,
    free_adults: i64,
    free_children: i64,
    is_admin: bool,
    no_age_distinction: bool,
) -> String {
    let mut header = format!(
        "\n \n{}\nНачало: {}.",
        event_title(&s.event),
        ts(s.event.ts)
    );
    if is_admin {
        header.push_str(&format!(
            " Мероприятие {} / {}({})",
            s.event.id, s.event.max_adults, s.event.max_children
        ));
    }

    if s.state == EventState::Open {
        if no_age_distinction {
            header.push_str(&format!(
                " Свободные места: {}",
                free_adults + free_children,
            ));
        } else {
            header.push_str(&format!("\nВзрослые свободные места: {}", free_adults));
            header.push_str(&format!("\nДетские свободные места: {}", free_children));
        }
    } else {
        header.push_str(" Запись остановлена.");
    }
    header
}

pub fn participants(
    s: &EventStats,
    participants: &Vec<Participant>,
    is_admin: bool,
    no_age_distinction: bool,
) -> String {
    let mut list = "".to_string();
    if participants.len() != 0 {
        list.push_str(&format!(
            "\n\nЗаписались {}({}):",
            s.adults.reserved, s.children.reserved
        ));
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
                    entry.push_str(&format!(" {}({})", p.adults, p.children));
                }
                if let Some(a) = &p.attachment {
                    entry.push_str(&format!(" {}", a));
                }
                entry
            })
            .collect::<String>(),
    );
    list
}

pub fn messages(
    conn: &Client,
    s: &EventStats,
    event_id: u64,
    is_admin: bool,
) -> Option<String> {
    let waiting_list = if is_admin {
        None
    } else {
        Some((s.adults.my_reservation == 0 && s.children.my_reservation == 0) as u64)
    };

    if let Ok(messages) = db::get_group_messages(conn, event_id, waiting_list) {
        if messages.len() > 0 {
            let formatted_list: String = messages
                .iter()
                .map(|msg| {
                    if waiting_list.is_some() {
                        format!("\n{}, {}:\n{}\n", msg.sender, ts(msg.ts), msg.text)
                    } else {
                        format!(
                            "\n{}, {} ({}):\n{}\n",
                            msg.sender,
                            ts(msg.ts),
                            if msg.waiting_list == 0 {
                                "для забронировавших"
                            } else {
                                "для списка ожидания"
                            },
                            msg.text
                        )
                    }
                })
                .collect();
            return Some(format!(
                "\n\n<b>Cообщения по мероприятию</b>{}",
                formatted_list
            ));
        }
    }
    None
}

#[test]
fn test_format() {
    assert_eq!(ts(1650445814), "20.04 09:10");
}
