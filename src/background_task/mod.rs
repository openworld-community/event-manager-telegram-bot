mod send_notifications;


use std::future::Future;
use crate::background_task::send_notifications::send_notifications;
use crate::configuration::config::Config;
use sea_orm::DatabaseConnection;
use std::time::Duration;
use futures::future::MaybeDone;

use teloxide::Bot;
use tokio::macros::support::maybe_done;

use tokio::sync::Mutex;
use std::sync::Arc;

pub async fn perform_background_task(bot: &Bot, config: &Config, connection: &DatabaseConnection) {
    let mut next_break = tokio::time::Instant::now() + Duration::from_millis(1000);
    let is_stop = Arc::new(Mutex::new(false));

    {
        let is_stop = is_stop.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            let mut locked = is_stop.lock().await;
            *locked = true;
        });
    }

    loop {
        let is_stop = {
            *is_stop.lock().await
        };

        if is_stop {
            break;
        }

        tokio::time::sleep_until(next_break).await;

        let (notifications, batch_contains_waiting_list_prompt) =
            send_notifications(config, bot, &connection)
                .await
                .unwrap();

        let duration = if notifications > 0 && batch_contains_waiting_list_prompt == false {
            1000
        } else {
            30_000
        };

        next_break = tokio::time::Instant::now() + Duration::from_millis(duration);
    }
}
