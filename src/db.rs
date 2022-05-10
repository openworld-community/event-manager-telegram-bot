use crate::util;
use fallible_streaming_iterator::FallibleStreamingIterator;
use rusqlite::{params, Connection, Result};
use std::collections::HashSet;

#[cfg(test)]
use std::println as trace;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct Event {
    pub id: i64,
    pub name: String,
    pub link: String,
    pub max_adults: i64,
    pub max_children: i64,
    pub max_adults_per_reservation: i64,
    pub max_children_per_reservation: i64,
    pub ts: i64,
    pub remind: i64,
}

pub struct EventState {
    pub event: Event,
    pub adults: i64,
    pub children: i64,
    pub my_adults: i64,
    pub my_children: i64,
    pub my_wait_adults: i64,
    pub my_wait_children: i64,
    pub state: i64,
}

pub struct Participant {
    pub user_id: i64,
    pub user_name1: String,
    pub user_name2: String,
    pub adults: i64,
    pub children: i64,
    pub attachment: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Reminder {
    pub event_id: i64,
    pub name: String,
    pub link: String,
    pub ts: i64,
    pub user_id: i64,
}

pub struct User {
    pub id: i64,
    pub user_name1: String,
    pub user_name2: String,
}

pub struct EventDB {
    pub conn: Connection,
}

impl EventDB {
    pub fn add_event(&self, e: Event) -> Result<i64, rusqlite::Error> {
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
                self.conn.execute(
                    "INSERT INTO alarms (event, remind) VALUES (?1, ?2)",
                    params![event_id, e.remind],
                )?;
                return Ok(event_id);
            }
        }
        Ok(0)
    }

    pub fn delete_event(&self, event_id: i64) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("DELETE FROM reservations WHERE event=?1", params![event_id])?;
        self.conn
            .execute("DELETE FROM events WHERE id=?1", params![event_id])?;
        self.conn
            .execute("DELETE FROM alarms WHERE event=?1", params![event_id])?;
        self.conn
            .execute("DELETE FROM attachments WHERE event=?1", params![event_id])?;
        Ok(())
    }

    pub fn sign_up(
        &self,
        event_id: i64,
        user: i64,
        user_name1: &str,
        user_name2: &str,
        adults: i64,
        children: i64,
        wait: i64,
        ts: i64,
    ) -> Result<(usize, bool), rusqlite::Error> {
        let s = self.get_event(event_id, user)?;

        if ts > s.event.ts {
            trace!("The even has already begun: {} {}", ts, s.event.ts);
            return Ok((0, false));
        }

        if s.state != 0 {
            trace!("Event closed");
            return Ok((0, false));
        }

        // Check user limits
        if s.my_adults + s.my_wait_adults + adults > s.event.max_adults_per_reservation
            || s.my_children + s.my_wait_children + children > s.event.max_children_per_reservation
        {
            trace!(
                "Order threshold reached: {} {}",
                s.my_adults + adults,
                s.my_children + children
            );
            return Ok((0, false));
        }

        // Check event limits
        let (vacant_adults, vacant_children) = self.get_vacancies(event_id)?;
        if wait == 0 && (adults > vacant_adults || children > vacant_children) {
            trace!(
                "Event threshold reached: {} {}",
                vacant_adults,
                vacant_children
            );
            return Ok((0, false));
        }

        let (waiting_list, black_listed) = match self.is_in_black_list(user) {
            Ok(v) => {
                if v {
                    (1, true)
                } else {
                    (wait, false)
                }
            }
            _ => (wait, false),
        };

        Ok((self.conn.execute(
            "INSERT INTO reservations (event, user, user_name1, user_name2, adults, children, waiting_list, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![event_id, user, user_name1, user_name2, adults, children, waiting_list, ts],
        )?, black_listed))
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

        let html_safe: String = attachment.chars().filter_map(
            |a| match a.is_alphanumeric() || a.is_ascii_whitespace() || a == ',' || a == '.' || a == ':' || a == '-'  { true => Some(a), false => Some(' ')} ).collect();

        let s = self.get_event(event_id, user)?;
        if s.my_adults > 0 || s.my_wait_adults > 0 || s.my_children > 0 || s.my_wait_children > 0 {
            self.conn.execute(
                "INSERT INTO attachments (event, user, attachment) VALUES (?1, ?2, ?3) ON CONFLICT (event, user) DO \
                UPDATE SET attachment=excluded.attachment",
                params![event_id, user, html_safe],
            )
        } else {
            Ok(0)
        }
    }

    pub fn cancel(
        &self,
        event_id: i64,
        user: i64,
        adults: i64,
    ) -> Result<HashSet<i64>, rusqlite::Error> {
        self.conn.execute(
            "DELETE FROM reservations WHERE id IN (SELECT id FROM reservations WHERE event=?1 AND user=?2 AND adults = ?3 ORDER BY waiting_list DESC LIMIT 1)",
            params![event_id, user, adults],
        )?;
        self.process_waiting_list(event_id, user)
    }

    pub fn wontgo(&self, event_id: i64, user: i64) -> Result<HashSet<i64>, rusqlite::Error> {
        self.conn.execute(
            "DELETE FROM reservations WHERE event=?1 AND user=?2",
            params![event_id, user],
        )?;
        self.process_waiting_list(event_id, user)
    }

    pub fn process_waiting_list(
        &self,
        event_id: i64,
        user_id: i64,
    ) -> Result<HashSet<i64>, rusqlite::Error> {
        let mut update: HashSet<i64> = HashSet::new();
        let (mut vacant_adults, mut vacant_children) = self.get_vacancies(event_id)?;

        if vacant_adults > 0 || vacant_children > 0 {
            let mut stmt = self.conn.prepare(
                "SELECT id, user, adults, children FROM reservations WHERE event = ?1 AND waiting_list = 1 AND user != ?2 ORDER BY ts"
            )?;
            let mut rows = stmt.query(params![event_id, user_id])?;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let user: i64 = row.get(1)?;
                let adults: i64 = row.get(2)?;
                let children: i64 = row.get(3)?;

                if adults > 0 && vacant_adults > 0 {
                    self.conn.execute(
                        "UPDATE reservations SET waiting_list = 0 WHERE id = ?1",
                        params![id],
                    )?;
                    update.insert(user);
                    vacant_adults -= 1;
                } else if children > 0 && vacant_children > 0 {
                    self.conn.execute(
                        "UPDATE reservations SET waiting_list = 0 WHERE id = ?1",
                        params![id],
                    )?;
                    update.insert(user);
                    vacant_children -= 1;
                }
            }
        }
        Ok(update)
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

    pub fn get_events(&self, user: i64) -> Result<Vec<EventState>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "select a.*, b.my_adults, b.my_children FROM \
            (SELECT events.id, events.name, events.link, events.max_adults, events.max_children, events.max_adults_per_reservation, events.max_children_per_reservation, events.ts, r.adults, r.children, events.state FROM events \
            LEFT JOIN (SELECT sum(adults) as adults, sum(children) as children, event FROM reservations WHERE waiting_list = 0 GROUP BY event) as r ON events.id = r.event) as a \
            LEFT JOIN (SELECT sum(adults) as my_adults, sum(children) as my_children, event FROM reservations WHERE user = ?1 GROUP BY event) as b ON a.id = b.event order by a.ts"
        )?;
        let mut rows = stmt.query([user])?;
        let mut res = Vec::new();
        while let Some(row) = rows.next()? {
            res.push(EventState {
                event: Event {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    link: row.get(2)?,
                    max_adults: row.get(3)?,
                    max_children: row.get(4)?,
                    max_adults_per_reservation: row.get(5)?,
                    max_children_per_reservation: row.get(6)?,
                    ts: row.get(7)?,
                    remind: 0,
                },
                adults: match row.get(8) {
                    Ok(v) => v,
                    Err(_) => 0,
                },
                children: match row.get(9) {
                    Ok(v) => v,
                    Err(_) => 0,
                },
                state: row.get(10)?,
                my_adults: match row.get(11) {
                    Ok(v) => v,
                    Err(_) => 0,
                },
                my_children: match row.get(12) {
                    Ok(v) => v,
                    Err(_) => 0,
                },
                my_wait_adults: 0,
                my_wait_children: 0,
            });
        }
        Ok(res)
    }

    pub fn get_event(&self, event_id: i64, user: i64) -> Result<EventState, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "select a.*, b.my_adults, b.my_children, c.my_wait_adults, c.my_wait_children FROM \
            (SELECT events.id, events.name, events.link, events.max_adults, events.max_children, events.max_adults_per_reservation, events.max_children_per_reservation, events.ts, r.adults, r.children, events.state FROM events \
            LEFT JOIN (SELECT sum(adults) as adults, sum(children) as children, event FROM reservations WHERE waiting_list = 0 GROUP BY event) as r ON events.id = r.event) as a \
            LEFT JOIN (SELECT sum(adults) as my_adults, sum(children) as my_children, event FROM reservations WHERE waiting_list = 0 AND user = ?1 GROUP BY event) as b ON a.id = b.event \
            LEFT JOIN (SELECT sum(adults) as my_wait_adults, sum(children) as my_wait_children, event FROM reservations WHERE waiting_list = 1 AND user = ?1 GROUP BY event) as c ON a.id = c.event WHERE a.id = ?2"

        )?;
        let mut rows = stmt.query([user, event_id])?;
        if let Some(row) = rows.next()? {
            Ok(EventState {
                event: Event {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    link: row.get(2)?,
                    max_adults: row.get(3)?,
                    max_children: row.get(4)?,
                    max_adults_per_reservation: row.get(5)?,
                    max_children_per_reservation: row.get(6)?,
                    ts: row.get(7)?,
                    remind: 0,
                },
                adults: match row.get(8) {
                    Ok(v) => v,
                    Err(_) => 0,
                },
                children: match row.get(9) {
                    Ok(v) => v,
                    Err(_) => 0,
                },
                state: row.get(10)?,
                my_adults: match row.get(11) {
                    Ok(v) => v,
                    Err(_) => 0,
                },
                my_children: match row.get(12) {
                    Ok(v) => v,
                    Err(_) => 0,
                },
                my_wait_adults: match row.get(13) {
                    Ok(v) => v,
                    Err(_) => 0,
                },
                my_wait_children: match row.get(14) {
                    Ok(v) => v,
                    Err(_) => 0,
                },
            })
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
    ) -> Result<Vec<Participant>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT a.*, b.attachment FROM (SELECT sum(adults) as adults, sum(children) as children, user, user_name1, user_name2, event, ts FROM reservations WHERE waiting_list = ?1 AND event = ?2 group by event, user ORDER BY ts) as a \
            LEFT JOIN attachments as b ON a.event = b.event and a.user = b.user"
        )?;
        let mut rows = stmt.query([waiting_list, event_id])?;
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

    pub fn get_user_reminders(&self, ts: i64) -> Result<Vec<Reminder>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.name, a.link, a.ts, c.user FROM events as a JOIN alarms as b ON a.id = b.event 
            JOIN (SELECT user, event FROM reservations WHERE waiting_list = 0 GROUP BY event) as c ON a.id = c.event WHERE b.remind < ?1"
        )?;
        let mut rows = stmt.query([ts])?;
        let mut res = Vec::new();
        while let Some(row) = rows.next()? {
            res.push(Reminder {
                event_id: row.get(0)?,
                name: row.get(1)?,
                link: row.get(2)?,
                ts: row.get(3)?,
                user_id: row.get(4)?,
            });
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

    pub fn clear_user_reminders(&self, ts: i64) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("DELETE FROM alarms WHERE remind < ?1", params![ts])?;
        Ok(())
    }

    pub fn clear_old_events(&self, ts: i64) -> Result<(), rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT id FROM events WHERE ts < ?1")?;
        let mut rows = stmt.query([ts - util::get_seconds_before_midnight(ts)])?;
        while let Some(row) = rows.next()? {
            let event_id: i64 = row.get(0)?;
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
                            "CREATE TABLE alarms (
                                event           INTEGER NOT NULL,
                                remind          INTEGER NOT NULL
                                )",
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
                                ts              INTEGER NOT NULL
                                )",
                            [],
                        )?;
                    }
                }
                _ => {}
            },
            _ => {
                error!("Failed to query db.");
            }
        }
        drop(stmt);
        Ok(EventDB { conn })
    }

    pub fn add_to_black_list(&self, user: i64) -> Result<(), rusqlite::Error> {
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

        self.conn.execute(
            "INSERT INTO black_list (user, user_name1, user_name2, ts) VALUES (?1, ?2, ?3, ?4)",
            params![user, user_name1, user_name2, util::get_unix_time()],
        )?;

        Ok(())
    }
    pub fn remove_from_black_list(&self, user: i64) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("DELETE FROM black_list WHERE user=?1", params![user])?;
        Ok(())
    }
    pub fn get_black_list(&self) -> Result<Vec<User>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT * FROM black_list")?;
        let mut rows = stmt.query([])?;
        let mut res = Vec::new();
        while let Some(row) = rows.next()? {
            res.push(User {
                id: row.get(0)?,
                user_name1: row.get(1)?,
                user_name2: row.get(2)?,
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
}
