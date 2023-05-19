use std::{fs::File, io::prelude::*};
use std::collections::HashSet;
use clap::ArgMatches;
use chrono::DateTime;

pub fn get_args<'a>() -> ArgMatches<'a> {
    clap::App::new("event-manager-telegram-bot")
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
        .get_matches()
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct RawConfiguration {
    pub telegram_bot_token: String,
    pub payment_provider_token: String,
    pub admin_ids: String,
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
    pub mailing_hours: String,
}

impl RawConfiguration {
    pub fn parse_mailing_hours(&self) -> Result<(u64, u64), String> {
        let parts: Vec<&str> = self.mailing_hours.split('.').collect();
        if parts.len() != 3 {
            return Err("Wrong mailing hours format.".to_string());
        }
        match (
            DateTime::parse_from_str(&format!("2022-07-06 {}", parts[0]), "%Y-%m-%d %H:%M  %z"),
            DateTime::parse_from_str(&format!("2022-07-06 {}", parts[2]), "%Y-%m-%d %H:%M  %z"),
        ) {
            (Ok(from), Ok(to)) => {
                let mailing_hours_from = (from.timestamp() % 86400) as u64;
                let mailing_hours_to = (to.timestamp() % 86400) as u64;
                Ok((mailing_hours_from, mailing_hours_to))
            }
            _ => Err("Failed to farse mailing hours.".to_string()),
        }
    }

    pub fn parse_admins(&self) -> HashSet<u64> {
        self.admin_ids
            .split(',')
            .into_iter()
            .filter_map(|id| id.parse::<u64>().ok())
            .collect()
    }
}

pub fn parse_config_file(path: &str) -> RawConfiguration {
    let mut f = File::open(path).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    toml::from_str::<RawConfiguration>(&contents)
        .map_err(|e| format!("Error loading configuration: {}", e))
        .unwrap()
}


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
}

impl From<RawConfiguration> for Config {
    fn from(value: RawConfiguration) -> Self {
        let mailing_hours = value.parse_mailing_hours().unwrap();
        Config {
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
        }
    }
}

pub fn get_config() -> Config {
    let args = get_args();
    let path = args.value_of("config").unwrap();
    Config::from(parse_config_file(path))
}

