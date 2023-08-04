use serde::{Deserialize, Serialize};
use serde_compact::compact;

#[compact]
#[derive(Serialize, Deserialize, Clone)]
pub enum CallbackQuery {
    EventList {
        offset: u64,
    },
    Event {
        event_id: u64,
        offset: u64,
    },
    SignUp {
        event_id: u64,
        is_adult: bool,
        wait: bool,
    },
    Cancel {
        event_id: u64,
        is_adult: bool,
    },
    WontGo {
        event_id: u64,
    },
    ShowWaitingList {
        event_id: u64,
        offset: u64,
    },
    ShowPresenceList {
        event_id: u64,
        offset: u64,
    },
    ConfirmPresence {
        event_id: u64,
        user_id: u64,
        offset: u64,
    },
    PaidEvent {
        event_id: u64,
        adults: u64,
        children: u64,
        offset: u64,
    },
    SendInvoice {
        event_id: u64,
        adults: u64,
        children: u64,
    },

    // admin callbacks
    ChangeEventState {
        event_id: u64,
        state: u64,
    },
    ShowBlackList {
        offset: u64,
    },
    RemoveFromBlackList {
        user_id: u64,
    },
    ConfirmRemoveFromBlackList {
        user_id: u64,
    },
}
