use lru::LruCache;
use std::collections::HashSet;
use telegram_bot::UserId;

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Configuration {
    pub telegram_bot_token: String,
    pub admin_ids: String,
    pub public_lists: bool,
    pub automatic_blacklisting: bool,
    pub drop_events_after_hours: i64,
    pub delete_from_black_list_after_days: i64,
    pub too_late_to_cancel_hours: i64,
    pub cleanup_old_events: bool,
    pub event_list_page_size: i64,
    pub event_page_size: i64,
    pub presence_page_size: i64,
    pub cancel_future_reservations_on_ban: bool,
    pub support: String,
    pub help: String,
    pub limit_bulk_notifications_per_second: i64,
    pub mailing_hours: String,
    pub mailing_hours_from: Option<i64>,
    pub mailing_hours_to: Option<i64>,
}

pub type EventId = i64;

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

pub struct Participant {
    pub user_id: i64,
    pub user_name1: String,
    pub user_name2: String,
    pub adults: i64,
    pub children: i64,
    pub attachment: Option<String>,
}

pub struct Presence {
    pub user_id: i64,
    pub user_name1: String,
    pub user_name2: String,
    pub reserved: i64,
    pub attachment: Option<String>,
}

pub struct MessageBatch {
    pub message_id: i64,
    pub event_id: i64,
    pub sender: String,
    pub message_type: MessageType,
    pub waiting_list: i64,
    pub text: String,
    pub recipients: Vec<i64>,
}

pub struct DialogState {
    current_user_events: LruCache<UserId, EventId>,
}

impl DialogState {
    pub fn new(user_cache_size: usize) -> Self {
        DialogState {
            current_user_events: LruCache::new(user_cache_size),
        }
    }
    pub fn get_current_user_event(&mut self, user: &UserId) -> Option<&EventId> {
        self.current_user_events.get(user)
    }
    pub fn set_current_user_event(&mut self, user: UserId, event: EventId) {
        self.current_user_events.put(user, event);
    }
}

#[derive(FromPrimitive, ToPrimitive, PartialEq)]
pub enum MessageType {
    Direct = 0,
    Reminder = 1,
    WaitingListPrompt = 2,
}
