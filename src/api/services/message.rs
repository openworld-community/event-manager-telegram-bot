use crate::api::services::message_outbox::create_message_outbox;
use chrono::Utc;
use entity::message::{ActiveModel, Column, Entity};
use entity::new_types::MessageType;
use entity::{message, message_outbox};
use sea_orm::prelude::*;
use sea_orm::{ActiveValue, DeleteResult};

pub async fn delete_enqueued_messages<C>(
    event_id: &i32,
    message_type: &MessageType,
    con: &C,
) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    let res: DeleteResult = Entity::delete_many()
        .filter(
            Column::Event
                .eq(*event_id)
                .and(Column::Type.eq(message_type.clone())),
        )
        .exec(con)
        .await?;

    Ok(res.rows_affected)
}

#[derive(Debug)]
pub struct ResultCreateMessage {
    pub message: message::Model,
    pub message_outbox: message_outbox::Model,
}

pub async fn create_message<C>(
    event_id: i32,
    sender: &str,
    waiting_list: i32,
    message_type: MessageType,
    text: &str,
    send_at: chrono::DateTime<Utc>,
    con: &C,
) -> Result<ResultCreateMessage, DbErr>
where
    C: ConnectionTrait,
{
    let ac = ActiveModel {
        id: ActiveValue::NotSet,
        event: ActiveValue::Set(event_id),
        r#type: ActiveValue::Set(message_type),
        sender: ActiveValue::Set(sender.to_string()),
        waiting_list: ActiveValue::Set(waiting_list),
        text: ActiveValue::Set(text.to_string()),
        ts: ActiveValue::Set(Utc::now().naive_utc()),
    };

    let message = ac.insert(con).await?;

    Ok(ResultCreateMessage {
        message_outbox: create_message_outbox(message.id, send_at, con).await?,
        message,
    })
}
