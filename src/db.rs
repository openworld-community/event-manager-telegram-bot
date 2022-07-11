use crate::types::{Event, EventState, MessageBatch, MessageType, Participant, Presence, User};
use crate::util::{self, format_ts, get_unix_time};
use fallible_streaming_iterator::FallibleStreamingIterator;
use rusqlite::{params, Connection, Result, Row};
use std::collections::HashSet;

#[cfg(test)]
use std::println as trace;

#[cfg(test)]
mod tests;

pub struct Counter {
    pub reserved: i64,
    pub my_reservation: i64,
    pub my_waiting: i64,
}

impl Counter {
    pub fn new(
        reserved: Result<i64, rusqlite::Error>,
        my_reservation: Result<i64, rusqlite::Error>,
        my_waiting: Result<i64, rusqlite::Error>,
    ) -> Result<Counter, rusqlite::Error> {
        Ok(Counter {
            reserved: match reserved {
                Ok(v) => v,
                Err(_) => 0,
            },
            my_reservation: match my_reservation {
                Ok(v) => v,
                Err(_) => 0,
            },
            my_waiting: match my_waiting {
                Ok(v) => v,
                Err(_) => 0,
            },
        })
    }
}

pub struct EventStats {
    pub event: Event,
    pub adults: Counter,
    pub children: Counter,
    pub state: EventState,
}

impl EventStats {
    pub fn new(row: &Row) -> Result<EventStats, rusqlite::Error> {
        let state: i64 = row.get("state")?;
        Ok(EventStats {
            event: Event {
                id: row.get("id")?,
                name: row.get("name")?,
                link: row.get("link")?,
                max_adults: row.get("max_adults")?,
                max_children: row.get("max_children")?,
                max_adults_per_reservation: row.get("max_adults_per_reservation")?,
                max_children_per_reservation: row.get("max_children_per_reservation")?,
                ts: row.get("ts")?,
                remind: 0,
            },
            adults: Counter::new(
                row.get("adults"),
                row.get("my_adults"),
                row.get("my_wait_adults"),
            )?,
            children: Counter::new(
                row.get("children"),
                row.get("my_children"),
                row.get("my_wait_children"),
            )?,
            state: match state {
                0 => EventState::Open,
                _ => EventState::Closed,
            },
        })
    }
}

pub struct EventDB {
    conn: Connection,
}

impl EventDB {
    pub fn add_event(&self, e: Event) -> Result<i64, rusqlite::Error> {
        if e.id == 0 {
            let res = self.conn.execute(
                "INSERT INTO events (name, link, max_adults, max_children, max_adults_per_reservation, max_children_per_reservation, ts, remind) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![e.name, e.link, e.max_adults, e.max_children, e.max_adults_per_reservation, e.max_children_per_reservation, e.ts, e.remind],
            )?;
            if res > 0 {
                let mut stmt = self
                    .conn
                    .prepare("SELECT id FROM events WHERE name = ?1 AND link = ?2 AND ts = ?3")?;
                let mut rows = stmt.query(params![e.name, e.link, e.ts])?;
                if let Some(row) = rows.next()? {
                    let event_id: i64 = row.get(0)?;

                    let text = format!("\nЗдравствуйте!\nНе забудьте, пожалуйста, что вы записались на\n<a href=\"{}\">{}</a>\
                                \nНачало: {}\n\
                                <b>ВНИМАНИЕ!</b> Если вы не сможете прийти и не отмените бронь заблаговременно, то не сможете больше бронировать. Извините, но бесплатные билеты не должны пропадать.\n",
                                e.link, e.name, format_ts(e.ts), );
                    self.enqueue_message(
                        event_id,
                        "Bot",
                        0,
                        MessageType::Reminder,
                        &text,
                        e.remind,
                    )?;
                    return Ok(event_id);
                }
            }
        } else {
            self.conn.execute(
                "UPDATE events SET name = ?1, link = ?2, max_adults = ?3, max_children = ?4, max_adults_per_reservation = ?5, max_children_per_reservation = ?6, ts = ?7, remind = ?8 \
                    WHERE id = ?9",
                params![e.name, e.link, e.max_adults, e.max_children, e.max_adults_per_reservation, e.max_children_per_reservation, e.ts, e.remind, e.id],
            )?;
            // todo: update reminder
        }
        Ok(0)
    }

    pub fn enqueue_message(
        &self,
        event_id: i64,
        sender: &str,
        waiting_list: i64,
        message_type: MessageType,
        text: &str,
        send_at: i64,
    ) -> Result<(), rusqlite::Error> {
        debug!("enqueue message {} {}", util::get_unix_time(), send_at);
        self.conn.execute(
            "INSERT INTO messages (event, type, sender, waiting_list, text, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![event_id, message_type as i64, sender, waiting_list, text, util::get_unix_time()],
        )?;
        let mut stmt = self.conn.prepare("SELECT last_insert_rowid()")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let message_id: i64 = row.get(0)?;
            self.conn.execute(
                "INSERT INTO message_outbox (message, send_at) VALUES (?1, ?2)",
                params![message_id, send_at],
            )?;
        }
        Ok(())
    }

    pub fn prompt_waiting_list(&self, event_id: i64) -> Result<(), rusqlite::Error> {
        if self.have_vacancies(event_id)? == false {
            debug!("prompt_waiting_list - no tickets, event {}", event_id);
            return Ok(());
        }

        let send_at = get_unix_time() + 10; // give some time to finish multiple cancellations
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM messages WHERE event = ?1 AND type = ?2")?;
        let mut rows = stmt.query([event_id, MessageType::WaitingListPrompt as i64])?;
        if let Some(row) = rows.next()? {
            let message_id: i64 = row.get("id")?;
            self.conn.execute(
                "INSERT INTO message_outbox (message, send_at) VALUES (?1, ?2)",
                params![message_id, send_at],
            )?;
        } else {
            if let Ok(event_name) = self.get_event_name(event_id) {
                self.enqueue_message(
                    event_id,
                    "Bot",
                    1,
                    MessageType::WaitingListPrompt,
                    &format!("Кто-то отменил бронирование на мероприятие: \"{}\".\nВы можете попробовать записаться.", event_name),
                    send_at
                )?;
            }
        }
        Ok(())
    }

    pub fn blacklist_absent_participants(
        &self,
        event_id: i64,
        admins: &HashSet<i64>,
        cancel_future_reservations: bool,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "select r.*, p.user from (select event, user, user_name1, user_name2, count(user) as count from reservations where event = ?1 and waiting_list = 0 group by user) as r 
            left join presence as p on r.event = p.event and r.user = p.user"
        )?;
        let mut rows = stmt.query(params![event_id])?;
        let mut list: Vec<Presence> = Vec::new();
        let mut presence_checked = false;
        while let Some(row) = rows.next()? {
            let present: rusqlite::Result<i64> = row.get(5);
            if let Err(_) = present {
                list.push(Presence {
                    user_id: row.get(1)?,
                    user_name1: row.get(2)?,
                    user_name2: row.get(3)?,
                    reserved: row.get(4)?,
                    attachment: None,
                });
            } else {
                presence_checked = true;
            }
        }
        if presence_checked && list.len() > 0 {
            // Check at least one present.
            if let Ok(reason) = self.get_event_name(event_id) {
                list.iter()
                    .filter(|p| !admins.contains(&p.user_id))
                    .try_for_each(|p| {
                        self.ban_user(
                            p.user_id,
                            &p.user_name1,
                            &p.user_name2,
                            &reason,
                            cancel_future_reservations,
                        )
                    })?;
            } else {
                warn!("Failed to get event {}", event_id);
            }
        }
        Ok(())
    }

    pub fn get_ban_reason(&self, user_id: i64) -> Result<String, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT reason FROM black_list WHERE user = ?1")?;
        let mut rows = stmt.query([user_id])?;
        if let Some(row) = rows.next()? {
            let reason: String = row.get("reason")?;
            Ok(reason)
        } else {
            Ok("unknown user".to_string())
        }
    }

    pub fn delete_event(&self, event_id: i64) -> Result<(), rusqlite::Error> {
        if let Err(e) = self
            .conn
            .execute("DELETE FROM reservations WHERE event=?1", params![event_id])
        {
            error!("{}", e);
        }
        if let Err(e) = self
            .conn
            .execute("DELETE FROM events WHERE id=?1", params![event_id])
        {
            error!("{}", e);
        }
        if let Err(e) = self
            .conn
            .execute("DELETE FROM attachments WHERE event=?1", params![event_id])
        {
            error!("{}", e);
        }
        if let Err(e) = self
            .conn
            .execute("DELETE FROM presence WHERE event=?1", params![event_id])
        {
            error!("{}", e);
        }
        if let Err(e) = self.conn.execute(
            "DELETE FROM group_leaders WHERE event=?1",
            params![event_id],
        ) {
            error!("{}", e);
        }
        if let Err(e) = self
            .conn
            .execute("DELETE FROM messages WHERE event=?1", params![event_id])
        {
            error!("{}", e);
        }
        Ok(())
    }

    pub fn sign_up(
        &self,
        event_id: i64,
        user: &User,
        adults: i64,
        children: i64,
        wait: i64,
        ts: i64,
    ) -> anyhow::Result<(usize, bool)> {
        let user_id = user.id.into();
        let s = self.get_event(event_id, user_id)?;

        if ts > s.event.ts || (s.state != EventState::Open && user.is_admin == false) {
            return Err(anyhow::anyhow!("Запись остановлена."));
        }

        if let Ok(black_listed) = self.is_in_black_list(user_id) {
            if black_listed {
                return Ok((0, true));
            }
        }

        // Check conflicting time
        let mut stmt = self
            .conn
            .prepare("select events.id from events join reservations as r on events.id = r.event where events.ts = ?1 and r.user = ?2 and events.id != ?3")?;
        let mut rows = stmt.query(params![s.event.ts, user_id, s.event.id])?;
        if let Some(_) = rows.next()? {
            return Err(anyhow::anyhow!(
                "Вы уже записаны на другое мероприятие в это время."
            ));
        }

        // Check user limits
        if s.adults.my_reservation + s.adults.my_waiting + adults
            > s.event.max_adults_per_reservation
        {
            if s.adults.my_reservation + adults > s.event.max_adults_per_reservation {
                return Ok((0, false));
            } else {
                self.move_from_waiting_list(event_id, user_id, 1, 0)?;
                return Ok((1, false));
            }
        }
        if s.children.my_reservation + s.children.my_waiting + children
            > s.event.max_children_per_reservation
        {
            if s.children.my_reservation + children > s.event.max_children_per_reservation {
                return Ok((0, false));
            } else {
                self.move_from_waiting_list(event_id, user_id, 0, 1)?;
                return Ok((1, false));
            }
        }

        Ok((self.conn.execute(
            "INSERT INTO reservations (event, user, user_name1, user_name2, adults, children, waiting_list, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![event_id, user_id, user.user_name1, user.user_name2, adults, children, wait, ts],
        )?, false))
    }

    fn move_from_waiting_list(
        &self,
        event_id: i64,
        user_id: i64,
        adults: i64,
        children: i64,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute("UPDATE reservations SET waiting_list = 0  WHERE id in \
            (SELECT id FROM reservations where event = ?1 and user = ?2 and waiting_list = 1 and adults = ?3 and children = ?4 order by ts limit 1)",
            params![event_id, user_id, adults, children],
        )?;
        Ok(())
    }

    pub fn add_attachment(
        &self,
        event_id: i64,
        user: i64,
        attachment: &str,
    ) -> Result<usize, rusqlite::Error> {
        if attachment.len() > 256 {
            warn!("attachment too long");
            return Ok(0);
        }

        let html_safe: String = attachment
            .chars()
            .filter_map(|a| {
                match a.is_alphanumeric()
                    || a.is_ascii_whitespace()
                    || a == ','
                    || a == '.'
                    || a == ':'
                    || a == '-'
                {
                    true => Some(a),
                    false => Some(' '),
                }
            })
            .collect();

        let s = self.get_event(event_id, user)?;
        if s.adults.my_reservation > 0
            || s.adults.my_waiting > 0
            || s.children.my_reservation > 0
            || s.children.my_waiting > 0
        {
            self.conn.execute(
                "INSERT INTO attachments (event, user, attachment) VALUES (?1, ?2, ?3) ON CONFLICT (event, user) DO \
                UPDATE SET attachment=excluded.attachment",
                params![event_id, user, html_safe],
            )
        } else {
            Ok(0)
        }
    }

    pub fn cancel(&self, event_id: i64, user: i64, adults: i64) -> Result<(), rusqlite::Error> {
        let state_changed = self.have_vacancies(event_id)? == false;
        self.conn.execute(
            "DELETE FROM reservations WHERE id IN (SELECT id FROM reservations WHERE event=?1 AND user=?2 AND adults = ?3 ORDER BY waiting_list DESC LIMIT 1)",
            params![event_id, user, adults],
        )?;
        if state_changed {
            self.prompt_waiting_list(event_id)
        } else {
            Ok(())
        }
    }

    pub fn wontgo(&self, event_id: i64, user: i64) -> Result<(), rusqlite::Error> {
        let state_changed = self.have_vacancies(event_id)? == false;
        self.conn.execute(
            "DELETE FROM reservations WHERE event=?1 AND user=?2",
            params![event_id, user],
        )?;
        if state_changed {
            self.prompt_waiting_list(event_id)
        } else {
            Ok(())
        }
    }

    fn have_vacancies(&self, event_id: i64) -> Result<bool, rusqlite::Error> {
        let (vacant_adults, vacant_children) = self.get_vacancies(event_id)?;
        if vacant_adults + vacant_children > 0 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn get_vacancies(&self, event_id: i64) -> Result<(i64, i64), rusqlite::Error> {
        let mut vacant_adults: i64 = 0;
        let mut vacant_children: i64 = 0;
        let mut stmt = self.conn.prepare(
            "SELECT a.max_adults, a.max_children, b.adults, b.children, a.id FROM events as a \
            LEFT JOIN (SELECT sum(adults) as adults, sum(children) as children, event FROM reservations WHERE event = ?1 AND waiting_list = 0 group by event) as b \
            ON a.id = b.event WHERE id = ?1 group by id"
        )?;
        let mut rows = stmt.query(params![event_id])?;
        if let Some(row) = rows.next()? {
            let max_adults: i64 = row.get(0)?;
            let max_children: i64 = row.get(1)?;
            let reserved_adults: i64 = match row.get(2) {
                Ok(v) => v,
                Err(_) => 0,
            };
            let reserved_children: i64 = match row.get(3) {
                Ok(v) => v,
                Err(_) => 0,
            };
            vacant_adults = max_adults - reserved_adults;
            vacant_children = max_children - reserved_children;
        }
        Ok((vacant_adults, vacant_children))
    }

    pub fn get_attachment(
        &self,
        event_id: i64,
        user: i64,
    ) -> Result<Option<String>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT attachment FROM attachments WHERE event = ?1 AND user = ?2")?;
        let mut rows = stmt.query(params![event_id, user])?;
        if let Some(row) = rows.next()? {
            let attachment: String = row.get(0)?;
            Ok(Some(attachment))
        } else {
            Ok(None)
        }
    }

    pub fn get_events(
        &self,
        user: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<EventStats>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "select a.*, b.my_adults, b.my_children, c.my_wait_adults, c.my_wait_children FROM \
            (SELECT events.id, events.name, events.link, events.max_adults, events.max_children, events.max_adults_per_reservation, events.max_children_per_reservation, events.ts, r.adults, r.children, events.state FROM events \
            LEFT JOIN (SELECT sum(adults) as adults, sum(children) as children, event FROM reservations WHERE waiting_list = 0 GROUP BY event) as r ON events.id = r.event ORDER BY ts LIMIT ?2 OFFSET ?3) as a \
            LEFT JOIN (SELECT sum(adults) as my_adults, sum(children) as my_children, event FROM reservations WHERE waiting_list = 0 AND user = ?1 GROUP BY event) as b ON a.id = b.event \
            LEFT JOIN (SELECT sum(adults) as my_wait_adults, sum(children) as my_wait_children, event FROM reservations WHERE waiting_list = 1 AND user = ?1 GROUP BY event) as c ON a.id = c.event"
        )?;
        let mut rows = stmt.query([user, limit, offset * limit])?;
        let mut res = Vec::new();
        while let Some(row) = rows.next()? {
            res.push(EventStats::new(row)?);
        }
        Ok(res)
    }

    pub fn get_event(&self, event_id: i64, user: i64) -> Result<EventStats, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "select a.*, b.my_adults, b.my_children, c.my_wait_adults, c.my_wait_children FROM \
            (SELECT events.id, events.name, events.link, events.max_adults, events.max_children, events.max_adults_per_reservation, events.max_children_per_reservation, events.ts, r.adults, r.children, events.state FROM events \
            LEFT JOIN (SELECT sum(adults) as adults, sum(children) as children, event FROM reservations WHERE waiting_list = 0 GROUP BY event) as r ON events.id = r.event) as a \
            LEFT JOIN (SELECT sum(adults) as my_adults, sum(children) as my_children, event FROM reservations WHERE waiting_list = 0 AND user = ?1 GROUP BY event) as b ON a.id = b.event \
            LEFT JOIN (SELECT sum(adults) as my_wait_adults, sum(children) as my_wait_children, event FROM reservations WHERE waiting_list = 1 AND user = ?1 GROUP BY event) as c ON a.id = c.event WHERE a.id = ?2"

        )?;
        let mut rows = stmt.query([user, event_id])?;
        if let Some(row) = rows.next()? {
            Ok(EventStats::new(row)?)
        } else {
            Err(rusqlite::Error::InvalidParameterName(
                "Failed to find event".to_string(),
            ))
        }
    }

    pub fn get_event_name(&self, event_id: i64) -> Result<String, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT events.name, events.ts FROM events WHERE id = ?1")?;
        let mut rows = stmt.query([event_id])?;
        if let Some(row) = rows.next()? {
            let name: String = row.get("name")?;
            let ts: i64 = row.get("ts")?;
            Ok(format!("{} {}", format_ts(ts), name))
        } else {
            Err(rusqlite::Error::InvalidParameterName(
                "Failed to find event".to_string(),
            ))
        }
    }

    pub fn get_participants(
        &self,
        event_id: i64,
        waiting_list: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Participant>, rusqlite::Error> {
        let mut stmt;
        let mut rows = if limit == 0 {
            stmt = self.conn.prepare(
            "SELECT a.*, b.attachment FROM (SELECT sum(adults) as adults, sum(children) as children, user, user_name1, user_name2, event, ts FROM reservations WHERE waiting_list = ?1 AND event = ?2 group by event, user ORDER BY ts) as a \
            LEFT JOIN attachments as b ON a.event = b.event and a.user = b.user"
            )?;
            stmt.query([waiting_list, event_id])?
        } else {
            stmt = self.conn.prepare(
                "SELECT a.*, b.attachment FROM (SELECT sum(adults) as adults, sum(children) as children, user, user_name1, user_name2, event, ts FROM reservations WHERE waiting_list = ?1 AND event = ?2 group by event, user ORDER BY ts LIMIT ?3 OFFSET ?4) as a \
                LEFT JOIN attachments as b ON a.event = b.event and a.user = b.user"
                )?;
            stmt.query([waiting_list, event_id, limit, offset * limit])?
        };
        let mut res = Vec::new();
        while let Some(row) = rows.next()? {
            res.push(Participant {
                adults: row.get(0)?,
                children: row.get(1)?,
                user_id: row.get(2)?,
                user_name1: row.get(3)?,
                user_name2: row.get(4)?,
                attachment: match row.get(7) {
                    Ok(v) => Some(v),
                    Err(_) => None,
                },
            });
        }
        Ok(res)
    }

    pub fn get_presence_list(
        &self,
        event_id: i64,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Presence>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
                "select r.*, p.user, a.attachment from (select event, user, user_name1, user_name2, count(user) from reservations where event = ?1 and waiting_list = 0 group by user) as r \
                left join presence as p on r.event = p.event and r.user = p.user \
                left join attachments as a on r.event = a.event and r.user = a.user \
                where p.user IS NULL order by r.user_name1 LIMIT ?2 OFFSET ?3"
        )?;
        let mut rows = stmt.query([event_id, limit, offset * limit])?;
        let mut res = Vec::new();
        while let Some(row) = rows.next()? {
            res.push(Presence {
                user_id: row.get(1)?,
                user_name1: row.get(2)?,
                user_name2: row.get(3)?,
                reserved: row.get(4)?,
                attachment: match row.get(6) {
                    Ok(v) => Some(v),
                    Err(_) => None,
                },
            });
        }
        Ok(res)
    }

    pub fn confirm_presence(&self, event_id: i64, user_id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "insert into presence (event, user) values (?1, ?2)",
            params![event_id, user_id],
        )?;
        Ok(())
    }

    pub fn is_group_leader(&self, event_id: i64, user_id: i64) -> Result<bool, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT event FROM group_leaders WHERE event = ?1 AND user = ?2")?;
        let mut rows = stmt.query(params![event_id, user_id])?;
        if let Some(_) = rows.next()? {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn set_group_leader(&self, event_id: i64, user_id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "insert into group_leaders (event, user) values (?1, ?2)",
            params![event_id, user_id],
        )?;
        Ok(())
    }

    pub fn delete_reservation(&self, event_id: i64, user_id: i64) -> Result<(), rusqlite::Error> {
        let state_changed = self.have_vacancies(event_id)? == false;
        self.conn.execute(
            "delete from reservations where event = ?1 and user = ?2",
            params![event_id, user_id],
        )?;
        if state_changed {
            self.prompt_waiting_list(event_id)
        } else {
            Ok(())
        }
    }

    pub fn get_pending_messages(
        &self,
        ts: i64,
        mut max_messages: i64,
    ) -> Result<Vec<MessageBatch>, rusqlite::Error> {
        //debug!("get_pending_messages {}", ts);
        let mut stmt = self.conn.prepare(
            "SELECT m.*, o.send_at FROM message_outbox as o \
            JOIN messages as m ON o.message = m.id \
            WHERE o.send_at < ?1",
        )?;
        let mut rows = stmt.query([ts])?;
        let mut res = Vec::new();
        while let Some(row) = rows.next()? {
            let message_type: i64 = row.get("type")?;
            let batch = MessageBatch {
                message_id: row.get("id")?,
                event_id: row.get("event")?,
                sender: row.get("sender")?,
                message_type: num::FromPrimitive::from_i64(message_type).unwrap(),
                waiting_list: row.get("waiting_list")?,
                text: row.get("text")?,
                recipients: Vec::new(),
            };
            res.push(batch);

            let batch = res.last_mut().unwrap();
            let mut collect_users = true;
            if batch.message_type == MessageType::WaitingListPrompt
                && self.have_vacancies(batch.event_id)? == false
            {
                collect_users = false;
            }

            if collect_users {
                let mut stmt = self.conn.prepare(
                    "SELECT r.user, s.message as sent FROM \
                            (select user, ts from reservations WHERE event = ?1 AND waiting_list = ?2 GROUP BY user) as r 
                            LEFT JOIN (select user, message from message_sent where message = ?3) as s 
                            ON r.user = s.user
                            WHERE sent is null ORDER BY r.ts LIMIT ?4"
                )?;
                let mut rows = stmt.query([
                    batch.event_id,
                    batch.waiting_list,
                    batch.message_id,
                    max_messages,
                ])?;

                while let Some(row) = rows.next()? {
                    let recipient: i64 = row.get("user")?;
                    batch.recipients.push(recipient);
                    max_messages -= 1;
                    if max_messages == 0 {
                        return Ok(res);
                    }

                    if batch.message_type == MessageType::WaitingListPrompt {
                        break; // take not more than one at a time
                    }
                }
            }
            if batch.recipients.len() == 0 {
                // Done with the message.
                debug!("finished sending message {}", batch.message_id);
                self.conn.execute(
                    "DELETE FROM message_outbox WHERE message = ?1",
                    params![batch.message_id],
                )?;
                self.conn.execute(
                    "DELETE FROM message_sent WHERE message = ?1",
                    params![batch.message_id],
                )?;
            }
        }
        Ok(res)
    }

    pub fn get_last_reservation_event(&self, user_id: i64) -> Result<i64, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT event FROM reservations WHERE user=?1 ORDER BY ts DESC LIMIT 1")?;
        let mut rows = stmt.query([user_id])?;
        if let Some(row) = rows.next()? {
            let event_id: i64 = row.get(0)?;
            Ok(event_id)
        } else {
            Ok(0)
        }
    }

    pub fn clear_old_events(
        &self,
        ts: i64,
        automatic_blacklisting: bool,
        cancel_future_reservations: bool,
        admins: &HashSet<i64>,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT id FROM events WHERE ts < ?1")?;
        let mut rows = stmt.query([ts - util::get_seconds_before_midnight(ts)])?;
        while let Some(row) = rows.next()? {
            let event_id: i64 = row.get(0)?;
            if automatic_blacklisting {
                if let Err(e) =
                    self.blacklist_absent_participants(event_id, admins, cancel_future_reservations)
                {
                    error!("{}", e);
                }
            }
            self.delete_event(event_id)?;
        }
        Ok(())
    }

    pub fn open(path: &str) -> Result<EventDB, rusqlite::Error> {
        let conn = Connection::open(path)?;

        let mut stmt =
            conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='events'")?;
        match stmt.query([]) {
            Ok(rows) => match rows.count() {
                Ok(count) => {
                    if count == 0 {
                        conn.execute(
                            "CREATE TABLE events (
                                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                                name            TEXT NOT NULL,
                                link            TEXT NOT NULL,
                                max_adults      INTEGER NOT NULL,
                                max_children    INTEGER NOT NULL,
                                max_adults_per_reservation   INTEGER NOT NULL,
                                max_children_per_reservation INTEGER NOT NULL,
                                ts              INTEGER NOT NULL,
                                remind          INTEGER NOT NULL,
                                state           INTEGER default 0
                                )",
                            [],
                        )?;
                        conn.execute(
                            "CREATE TABLE reservations (
                                id              INTEGER PRIMARY KEY,
                                event           INTEGER NOT NULL,
                                user            INTEGER NOT NULL,
                                user_name1      TEXT NOT NULL,
                                user_name2      TEXT NOT NULL,
                                adults          INTEGER NOT NULL,
                                children        INTEGER NOT NULL,
                                waiting_list    INTEGER DEFAULT 0 NOT NULL,
                                ts              INTEGER NOT NULL
                                )",
                            [],
                        )?;
                        conn.execute(
                            "CREATE INDEX reservations_event_index ON reservations (event)",
                            [],
                        )?;
                        conn.execute(
                            "CREATE INDEX reservations_user_index ON reservations (user)",
                            [],
                        )?;

                        conn.execute(
                            "CREATE TABLE attachments (
                                event           INTEGER NOT NULL,
                                user            INTEGER NOT NULL,
                                attachment      TEXT NOT NULL
                                )",
                            [],
                        )?;
                        conn.execute(
                            "CREATE INDEX attachments_event_index ON attachments (event)",
                            [],
                        )?;
                        conn.execute("CREATE UNIQUE INDEX attachments_unique_event_user_idx ON attachments (event, user)", [])?;

                        conn.execute(
                            "CREATE TABLE black_list (
                                user            INTEGER PRIMARY KEY,
                                user_name1      TEXT NOT NULL,
                                user_name2      TEXT NOT NULL,
                                ts              INTEGER NOT NULL,
                                reason          TEXT default ''
                                )",
                            [],
                        )?;

                        conn.execute(
                            "CREATE TABLE presence (
                                event           INTEGER NOT NULL,
                                user            INTEGER NOT NULL
                                )",
                            [],
                        )?;
                        conn.execute("CREATE INDEX presence_event_index ON presence (event)", [])?;
                        conn.execute("CREATE UNIQUE INDEX presence_event_user_unique_idx ON presence (event, user)", [])?;

                        conn.execute(
                            "CREATE TABLE group_leaders (
                                event           INTEGER NOT NULL,
                                user            INTEGER NOT NULL
                                )",
                            [],
                        )?;
                        conn.execute("CREATE UNIQUE INDEX group_leaders_event_user_unique_idx ON presence (event, user)", [])?;

                        conn.execute(
                            "CREATE TABLE messages (
                                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                                event           INTEGER NOT NULL,
                                type            INTEGER NOT NULL,
                                sender          text NOT NULL,
                                waiting_list    INTEGER NOT NULL,
                                text            text NOT NULL,
                                ts              INTEGER NOT NULL
                              )",
                            [],
                        )?;
                        conn.execute("CREATE INDEX messages_event_index ON messages (event)", [])?;

                        conn.execute(
                            "CREATE TABLE message_outbox (
                                message         INTEGER NOT NULL,
                                send_at         INTEGER NOT NULL
                              )",
                            [],
                        )?;
                        conn.execute(
                            "CREATE TABLE message_sent (
                                message         INTEGER NOT NULL,
                                user            INTEGER NOT NULL,
                                ts              INTEGER NOT NULL
                              )",
                            [],
                        )?;
                    }
                }
                _ => panic!("DB is corrupt."),
            },
            _ => {
                error!("Failed to query db.");
            }
        }
        drop(stmt);
        Ok(EventDB { conn })
    }

    pub fn save_receipt(&self, message_id: i64, user: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO message_sent (message, user, ts) VALUES (?1, ?2, ?3)",
            params![message_id, user, util::get_unix_time()],
        )?;
        Ok(())
    }

    pub fn add_to_black_list(
        &self,
        user: i64,
        cancel_future_reservations: bool,
    ) -> Result<(), rusqlite::Error> {
        let mut user_name1 = user.to_string();
        let mut user_name2 = "".to_string();

        let mut stmt = self
            .conn
            .prepare("SELECT user_name1, user_name2 FROM reservations WHERE user = ?1 LIMIT 1")?;
        let mut rows = stmt.query([user])?;
        if let Some(row) = rows.next()? {
            user_name1 = row.get(0)?;
            user_name2 = row.get(1)?;
        }

        self.ban_user(
            user,
            &user_name1,
            &user_name2,
            "banned by admin",
            cancel_future_reservations,
        )
    }

    pub fn ban_user(
        &self,
        user: i64,
        user_name1: &str,
        user_name2: &str,
        reason: &str,
        cancel_future_reservations: bool,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO black_list (user, user_name1, user_name2, ts, reason) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![user, user_name1, user_name2, util::get_unix_time(), reason],
        )?;

        if cancel_future_reservations {
            if let Err(e) = self
                .conn
                .execute("DELETE FROM reservations where user = ?1", params![user])
            {
                warn!("{}", e);
            }
        }
        Ok(())
    }

    pub fn remove_from_black_list(&self, user: i64) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("DELETE FROM black_list WHERE user=?1", params![user])?;
        Ok(())
    }
    pub fn get_black_list(&self, offset: i64, limit: i64) -> Result<Vec<User>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM black_list order by user_name1 LIMIT ?1 OFFSET ?2")?;
        let mut rows = stmt.query([limit, offset * limit])?;
        let mut res = Vec::new();
        while let Some(row) = rows.next()? {
            let user_id: i64 = row.get(0)?;
            res.push(User {
                id: user_id.into(),
                user_name1: row.get(1)?,
                user_name2: row.get(2)?,
                is_admin: false,
            });
        }
        Ok(res)
    }

    pub fn is_in_black_list(&self, user: i64) -> Result<bool, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM black_list WHERE user = ?1")?;
        let mut rows = stmt.query([user])?;
        if let Some(_) = rows.next()? {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear_black_list(&self, ts: i64) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("DELETE FROM black_list WHERE ts < ?1", params![ts])?;
        Ok(())
    }

    pub fn change_event_state(&self, event_id: i64, state: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE events SET state = ?1 WHERE id = ?2",
            params![state, event_id],
        )?;
        Ok(())
    }

    pub fn set_event_limits(
        &self,
        event_id: i64,
        max_adults: i64,
        max_children: i64,
    ) -> Result<(), rusqlite::Error> {
        let state_changed = self.have_vacancies(event_id)? == false;
        self.conn.execute(
            "UPDATE events SET max_adults = ?1, max_children = ?2 WHERE id = ?3",
            params![max_adults, max_children, event_id],
        )?;
        if state_changed {
            self.prompt_waiting_list(event_id)
        } else {
            Ok(())
        }
    }

    pub fn get_messages(
        &self,
        event_id: i64,
        waiting_list: Option<i64>,
    ) -> Result<Option<String>, rusqlite::Error> {
        let mut messages = String::new();

        let mut stmt;
        let mut rows = if let Some(waiting_list) = waiting_list {
            stmt = self.conn.prepare(
                "SELECT sender, text, ts, waiting_list FROM messages WHERE event = ?1 AND type = 0 AND waiting_list = ?2 ORDER BY ts"
            )?;
            stmt.query(params![event_id, waiting_list])?
        } else {
            stmt = self.conn.prepare(
                "SELECT sender, text, ts, waiting_list FROM messages WHERE event = ?1 AND type = 0 ORDER BY ts",
            )?;
            stmt.query(params![event_id])?
        };
        while let Some(row) = rows.next()? {
            let sender: String = row.get(0)?;
            let text: String = row.get(1)?;
            let ts: i64 = row.get(2)?;
            // todo: remove after message format migration
            if sender.len() == 0 {
                continue;
            }
            if waiting_list.is_some() {
                messages.push_str(&format!("{}, {}:\n{}\n\n", sender, format_ts(ts), text));
            } else {
                let waiting_list: i64 = row.get(3)?;
                messages.push_str(&format!(
                    "{}, {} ({}):\n{}\n\n",
                    sender,
                    format_ts(ts),
                    if waiting_list == 0 {
                        "для забронировавших"
                    } else {
                        "для списка ожидания"
                    },
                    text
                ));
            }
        }
        if messages.len() > 0 {
            Ok(Some(messages))
        } else {
            Ok(None)
        }
    }
}
