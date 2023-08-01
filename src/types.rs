use deadpool_postgres::{ManagedConnection, ManagerConfig, Pool, PoolError, RecyclingMethod};
// use r2d2::PooledConnection;
// use r2d2_sqlite::SqliteConnectionManager;
use serde_compact::compact;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::configuration::config::Config;
use teloxide::types::UserId;

pub type DbPool = deadpool_postgres::Pool;
pub type Connection = deadpool_postgres::ManagedConnection;
//pub type EventId = u64;

#[derive(PartialEq)]
pub enum EventType {
    Announcement = 0,
    Free = 1,
    Paid = 2,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    pub name: String,
    pub link: String,
    pub max_adults: u64,
    pub max_children: u64,
    pub max_adults_per_reservation: u64,
    pub max_children_per_reservation: u64,
    pub ts: u64,
    pub remind: u64,
    pub adult_ticket_price: u64,
    pub child_ticket_price: u64,
    pub currency: String,
}

impl Event {
    pub fn get_type(&self) -> EventType {
        // todo: move to constructor
        if self.adult_ticket_price != 0 || self.child_ticket_price != 0 {
            EventType::Paid
        } else if self.max_adults != 0 || self.max_children != 0 {
            EventType::Free
        } else {
            EventType::Announcement
        }
    }
}

#[derive(PartialEq)]
pub enum EventState {
    Open,
    Closed,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct User {
    pub id: UserId,
    pub user_name1: String,
    pub user_name2: String,
    pub is_admin: bool,
}

impl User {
    pub fn new(u: &teloxide::types::User, admins: &HashSet<u64>) -> User {
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
            is_admin: admins.contains(&u.id.0),
        }
    }
}

pub struct Participant {
    pub user_id: u64,
    pub user_name1: String,
    pub user_name2: String,
    pub adults: u64,
    pub children: u64,
    pub attachment: Option<String>,
}

pub struct Presence {
    pub user_id: u64,
    pub user_name1: String,
    pub user_name2: String,
    pub reserved: u64,
    pub attachment: Option<String>,
}

pub struct MessageBatch {
    pub message_id: u64,
    pub event_id: u64,
    pub sender: String,
    pub message_type: MessageType,
    pub waiting_list: u64,
    pub text: String,
    pub is_paid: bool,
    pub recipients: Vec<u64>,
}

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum MessageType {
    Direct = 0,
    Reminder = 1,
    WaitingListPrompt = 2,
}

pub struct Context {
    pub config: Config,
    pub pool: DbPool,
    pub sign_up_mutex: Arc<Mutex<u64>>,
}

#[compact]
#[derive(Serialize, Deserialize, Clone)]
pub struct Booking {
    pub event_id: u64,
    pub adults: u64,
    pub children: u64,
    pub user_id: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OrderInfo {
    pub id: String,
    pub name: String,
    pub amount: u64,
}

pub enum ReservationState {
    Free = 0,
    PaymentPending = 1,
    PaymentCompleted = 2,
}
