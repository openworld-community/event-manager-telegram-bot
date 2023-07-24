use crate::configuration::config::Config;
use sea_orm::DatabaseConnection;
use std::time::Duration;
use teloxide::adaptors::AutoSend;
use teloxide::Bot;

pub async fn perform_background_task(
    _bot: AutoSend<Bot>,
    _config: &Config,
    _connection: &DatabaseConnection,
) {
    let mut next_break = tokio::time::Instant::now() + Duration::from_millis(1000);
    loop {
        tokio::time::sleep_until(next_break).await;

        // TODO: some background tasks

        next_break = tokio::time::Instant::now() + Duration::from_millis(5000);
    }
}
