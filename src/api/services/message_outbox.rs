use chrono::{DateTime, Utc};
use entity::message_outbox;
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, DbErr};

pub async fn create_message_outbox<C, ID>(
    message_id: ID,
    send_at: DateTime<Utc>,
    con: &C,
) -> Result<message_outbox::Model, DbErr>
where
    C: ConnectionTrait,
    ID: Into<i32>,
{
    let ac = message_outbox::ActiveModel {
        id: ActiveValue::NotSet,
        message: ActiveValue::Set(message_id.into()),
        send_at: ActiveValue::Set(send_at.naive_utc()),
    };

    ac.insert(con).await
}

