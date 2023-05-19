use chrono::DateTime;
use std::collections::HashSet;

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
