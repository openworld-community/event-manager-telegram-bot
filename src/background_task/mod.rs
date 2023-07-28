mod send_notifications;

use crate::background_task::send_notifications::send_notifications;
use crate::configuration::config::Config;
use sea_orm::DatabaseConnection;
use std::time::Duration;
use teloxide::adaptors::AutoSend;
use teloxide::Bot;

pub async fn perform_background_task(
    _bot: AutoSend<Bot>,
    config: &Config,
    connection: &DatabaseConnection,
) {
    let mut next_break = tokio::time::Instant::now() + Duration::from_millis(1000);
    loop {
        tokio::time::sleep_until(next_break).await;

        let (notifications, batch_contains_waiting_list_prompt) =
            send_notifications(config, connection).await.unwrap();

        let duration = if notifications > 0 && batch_contains_waiting_list_prompt == false {
            1000
        } else {
            30_000
        };

        next_break = tokio::time::Instant::now() + Duration::from_millis(duration);
    }
}
