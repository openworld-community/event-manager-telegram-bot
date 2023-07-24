use crate::configuration::raw_config::RawConfiguration;
use std::collections::HashSet;
use std::net::SocketAddr;

pub struct Config {
    pub telegram_bot_token: String,
    pub payment_provider_token: String,
    pub admins: HashSet<u64>,
    pub public_lists: bool,
    pub automatic_blacklisting: bool,
    pub drop_events_after_hours: u64,
    pub delete_from_black_list_after_days: u64,
    pub too_late_to_cancel_hours: u64,
    pub cleanup_old_events: bool,
    pub event_list_page_size: u64,
    pub event_page_size: u64,
    pub presence_page_size: u64,
    pub cancel_future_reservations_on_ban: bool,
    pub support: String,
    pub help: String,
    pub limit_bulk_notifications_per_second: u64,
    pub mailing_hours_from: u64,
    pub mailing_hours_to: u64,
    pub api_socket_address: SocketAddr,
    pub database_connection: String,
    pub db_protocol: String,
}

impl From<RawConfiguration> for Config {
    fn from(value: RawConfiguration) -> Self {
        let mailing_hours = value.parse_mailing_hours().unwrap();
        let database_connection = value.db_protocol() + &value.db_host() + ":" + &value.db_port() + "/" + &value.db_name();
        Config {
            api_socket_address: value.socket_address(),
            telegram_bot_token: value.telegram_bot_token.clone(),
            payment_provider_token: value.payment_provider_token.clone(),
            admins: value.parse_admins(),
            public_lists: value.public_lists,
            automatic_blacklisting: value.automatic_blacklisting,
            drop_events_after_hours: value.drop_events_after_hours,
            delete_from_black_list_after_days: value.delete_from_black_list_after_days,
            too_late_to_cancel_hours: value.too_late_to_cancel_hours,
            cleanup_old_events: value.cleanup_old_events,
            event_list_page_size: value.event_list_page_size,
            event_page_size: value.event_page_size,
            presence_page_size: value.presence_page_size,
            cancel_future_reservations_on_ban: value.cancel_future_reservations_on_ban,
            support: value.support,
            help: value.help,
            limit_bulk_notifications_per_second: value.limit_bulk_notifications_per_second,
            mailing_hours_from: mailing_hours.0,
            mailing_hours_to: mailing_hours.1,
            database_connection: value.database_connection,
            db_protocol: value.db_protocol,
        }
    }
}
