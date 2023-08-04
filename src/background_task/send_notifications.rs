use crate::api::services::message::get_pending_messages;
use crate::configuration::config::Config;
use chrono::Utc;
use sea_orm::{DatabaseConnection, DbErr, TransactionTrait};

fn is_mailing_time(cfg: &Config, current_time: &chrono::DateTime<Utc>) -> bool {
    let ts = current_time.timestamp();
    let seconds_from_midnight = ts % 86400;

    return seconds_from_midnight >= cfg.mailing_hours_from as i64
        && seconds_from_midnight < cfg.mailing_hours_to as i64;
}

pub async fn send_notifications(
    cfg: &Config,
    connection: &DatabaseConnection,
) -> Result<(i32, bool), DbErr> {
    let transaction = connection.begin().await?;

    let current_time = Utc::now();
    let notifications = 0;
    let batch_contains_waiting_list_prompt = false;

    if !is_mailing_time(cfg, &current_time) {
        return Ok((notifications, batch_contains_waiting_list_prompt));
    }

    let _messages = get_pending_messages(
        &current_time,
        cfg.limit_bulk_notifications_per_second as i32,
        &transaction,
    )
    .await?;

    transaction.commit().await?;

    return Ok((notifications, batch_contains_waiting_list_prompt));
}
