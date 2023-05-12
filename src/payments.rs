use crate::message_handler::CallbackQuery;
use crate::types::{
    Booking, Context, EventState, OrderInfo, ReservationState, User,
};
use crate::reply::*;
use crate::util::{get_unix_time};
use anyhow::anyhow;
use teloxide::types::{
    Currency, InlineKeyboardButton, PreCheckoutQuery, SuccessfulPayment,
};

use crate::db;
use crate::format;
use db::EventStats;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

/// Book tickets and wait for payment checkout.
pub fn pre_checkout(
    conn: &PooledConnection<SqliteConnectionManager>,
    user: &User,
    pre_checkout: &PreCheckoutQuery,
    _ctx: &Context,
) -> anyhow::Result<()> {
    // if pre_checkout.currency != Currency::EUR {
    //     return Err(anyhow!("Only EUR is currently accepted"));
    // }
    if let Some(_) = &pre_checkout.order_info.name {
        let booking: Booking = serde_json::from_str(&pre_checkout.invoice_payload)?;
        if booking.event_id == 0 {
            // Donation
            Ok(())
        } else {
            match db::sign_up(
                conn,
                booking.event_id,
                user,
                booking.adults,
                booking.children,
                0,
                get_unix_time(),
                pre_checkout.total_amount as u64,
            ) {
                Ok(_) => Ok(()),
                Err(e) => Err(anyhow!("{}", e)),
            }
        }
    } else {
        Err(anyhow!("Name not found"))
    }
}

/// Payment successful.
pub fn checkout(
    conn: &PooledConnection<SqliteConnectionManager>,
    payment: &SuccessfulPayment,
    _ctx: &Context,
) -> anyhow::Result<()> {
    if let Some(name) = &payment.order_info.name {
        let booking: Booking = serde_json::from_str(&payment.invoice_payload)?;
        if booking.event_id == 0 {
            // Donation
            // todo: save to donations table
            Ok(())
        } else {
            match db::checkout(
                conn,
                &booking,
                OrderInfo {
                    id: payment.telegram_payment_charge_id.to_owned(),
                    name: name.to_owned(),
                    amount: payment.total_amount as u64,
                },
            ) {
                Ok(_) => Ok(()),
                Err(e) => Err(anyhow!("{}", e)),
            }
        }
    } else {
        Err(anyhow!("Name not found"))
    }
}

pub fn show_paid_event(
    event_id: u64,
    adults: u64,
    children: u64,
    offset: u64,
    conn: &PooledConnection<SqliteConnectionManager>,
    user: &User,
    ctx: &Context,
) -> anyhow::Result<Reply> {
    match db::get_event(conn, event_id, user.id.0) {
        Ok(s) => {
            let free_adults = s.event.max_adults as i64 - s.adults.reserved as i64 - adults as i64;
            let free_children = s.event.max_children as i64 - s.children.reserved as i64 - children as i64;
            let no_age_distinction = s.event.max_adults == 0 || s.event.max_children == 0;
            let is_admin = ctx.admins.contains(&user.id.0);

            let (participants, participants_len) = if is_admin {
                let participants = db::get_participants(
                    conn,
                    event_id,
                    0,
                    offset,
                    ctx.config.event_page_size,
                    ReservationState::PaymentCompleted,
                )?;
                let len = participants.len();
                (Some(participants), len as u64)
            } else {
                (None, 0)
            };

            Ok(
                // header
                ReplyMessage::new(format::header(
                    &s,
                    free_adults,
                    free_children,
                    is_admin,
                    no_age_distinction,
                ))
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
                // footer
                .text(
                    if s.adults.my_reservation + s.children.my_reservation > 0 {
                        Some(format!("\n<b>Вы ранее купили: {}</b>", s.adults.my_reservation + s.children.my_reservation))
                    } else {
                        None
                    }        
                )
                // order
                .text(
                    if adults + children > 0 {
                        let mut order = "Сбор за бронирование: ".to_string();
                        if no_age_distinction {
                            order.push_str(&format!("{}", adults));
                        } else {
                            if adults > 0 {
                                order.push_str(&format!("{} взросл.", adults));
                            }
                            if children > 0 {
                                order.push_str(&format!(", {} детск.", children));
                            }
                        }
        
                        let total_amount = (adults * s.event.adult_ticket_price
                            + children * s.event.child_ticket_price)
                            as f32
                            / 100f32;
                        Some(format!(
                            "\n<b>{}, всего {} {}</b>",
                            order, total_amount, s.event.currency
                        ))
                    } else {
                        Some("\nВыберите необходимое количество билетов и нажмите \"К оплате\". Введённое имя будет на билете.".to_string())
                    }                    
                )
                // controls
                .keyboard(get_controls(
                    &s,
                    adults,
                    children,
                    offset,
                    free_adults,
                    free_children,
                    no_age_distinction,
                    is_admin,
                    user.id.0,
                    conn,
                )?)
                // pagination
                .pagination(
                    &CallbackQuery::PaidEvent {
                        event_id,
                        adults,
                        children,
                        offset: offset.saturating_sub(1),
                    },
                    &CallbackQuery::PaidEvent {
                        event_id,
                        adults,
                        children,
                        offset: offset + 1,
                    },
                    participants_len,
                    ctx.config.event_page_size,
                    offset,
                )?                
                .into()
            )
        }
        Err(e) => Err(anyhow!("Failed to fetch event: {}", e)),
    }
}

fn get_controls(
    s: &EventStats,
    adults: u64,
    children: u64,
    offset: u64,
    free_adults: i64,
    free_children: i64,
    no_age_distinction: bool,
    is_admin: bool,
    _user_id: u64,
    _conn: &PooledConnection<SqliteConnectionManager>,
) -> anyhow::Result<Vec<Vec<InlineKeyboardButton>>> {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let mut row: Vec<InlineKeyboardButton> = Vec::new();
    let event_id = s.event.id;

    if s.state == EventState::Open {
        if s.adults.my_reservation + adults < s.event.max_adults_per_reservation {
            if free_adults > 0 {
                row.push(InlineKeyboardButton::callback(
                    if no_age_distinction {
                        "Забронировать +1"
                    } else {
                        "Забронировать взрослый +1"
                    },
                    &serde_json::to_string(&CallbackQuery::PaidEvent {
                        event_id,
                        adults: adults + 1,
                        children,
                        offset,
                    })?,
                ));
            }
        }
        if adults > 0 {
            row.push(InlineKeyboardButton::callback(
                if no_age_distinction {
                    "Отменить -1"
                } else {
                    "Отменить взрослый -1"
                },
                &serde_json::to_string(&CallbackQuery::PaidEvent {
                    event_id,
                    adults: adults - 1,
                    children,
                    offset,
                })?,
            ));
        }
        keyboard.push(row);
        row = Vec::new();
        if s.children.my_reservation + children < s.event.max_children_per_reservation
        {
            if free_children > 0 {
                row.push(InlineKeyboardButton::callback(
                    if no_age_distinction {
                        "Забронировать +1"
                    } else {
                        "Забронировать детский +1"
                    },
                    &serde_json::to_string(&CallbackQuery::PaidEvent {
                        event_id,
                        adults,
                        children: children + 1,
                        offset,
                    })?,
                ));
            }
        }
        if children > 0 {
            row.push(InlineKeyboardButton::callback(
                if no_age_distinction {
                    "Отменить -1"
                } else {
                    "Отменить детский -1"
                },
                &serde_json::to_string(&CallbackQuery::PaidEvent {
                    event_id,
                    adults,
                    children: children - 1,
                    offset,
                })?,
            ));
        }
        keyboard.push(row);
    }
    row = Vec::new();
    row.push(InlineKeyboardButton::callback(
        "Список мероприятий",
        serde_json::to_string(&CallbackQuery::EventList { offset: 0 })?,
    ));

    if is_admin {
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
    }

    if adults + children > 0 {
        row.push(InlineKeyboardButton::callback(
            "К оплате",
            serde_json::to_string(&CallbackQuery::SendInvoice {
                event_id,
                adults,
                children,
            })?,
        ));
    }
    keyboard.push(row);

    Ok(keyboard)
}

pub fn prepare_invoice(
    event_id: u64,
    adults: u64,
    children: u64,
    conn: &PooledConnection<SqliteConnectionManager>,
    user: &User,
    _ctx: &Context,
) -> anyhow::Result<Reply> {
    match db::get_event(conn, event_id, user.id.0) {
        Ok(s) => {
            let no_age_distinction = s.event.max_adults == 0 || s.event.max_children == 0;
            if s.state != EventState::Open {
                Err(anyhow!("Event has been closed"))
            } else if s.adults.my_reservation + adults > s.event.max_adults_per_reservation
                || s.children.my_reservation + children > s.event.max_children_per_reservation
            {
                Err(anyhow!("Limits error"))
            } else {
                let mut title = "Билеты: ".to_string();
                if no_age_distinction {
                    title.push_str(&format!("{}", adults + children));
                } else {
                    if adults > 0 {
                        title.push_str(&format!("{} взросл. ", adults));
                    }
                    if children > 0 {
                        title.push_str(&format!(", {} детск.", children));
                    }
                }

                Ok(Reply::Invoice {
                    title,
                    description: format!("{} - {}", s.event.name, format::ts(s.event.ts)),
                    currency: s.event.currency,
                    amount: adults * s.event.adult_ticket_price
                        + children * s.event.child_ticket_price,
                    payload: serde_json::to_string(&Booking {
                        event_id,
                        adults,
                        children,
                        user_id: user.id.0,
                    })?,
                })
            }
        }
        Err(e) => Err(anyhow!("Failed to fetch event: {}", e)),
    }
}

pub fn donate(
    user: &User,
    amount: u64,
    _ctx: &Context,
) -> anyhow::Result<Reply> {
        Ok(Reply::Invoice {
            title: "Донат".to_string(),
            description: "Поддержать работу канала \"Венские Истории\"".to_string(),
            currency: "EUR".to_string(),
            amount,
            payload: serde_json::to_string(&Booking {
                event_id: 0,
                adults: 0,
                children: 0,
                user_id: user.id.0,
            })?,
        })
}