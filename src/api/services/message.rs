use crate::api::services::message_outbox::create_message_outbox;
use crate::api::services::reservation::get_vacancies;
use chrono::Utc;
use entity::message::{ActiveModel, Column, Entity};
use entity::new_types::MessageType;
use entity::{message, message_outbox, message_sent};
use futures::TryStreamExt;
use sea_orm::prelude::*;
use sea_orm::{
    ActiveValue, DeleteResult, FromQueryResult, SelectModel, SelectorRaw, Statement, StreamTrait,
};

pub async fn delete_enqueued_messages<C>(
    event_id: &i32,
    message_type: Option<&MessageType>,
    con: &C,
) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    let filter = match message_type {
        None => Column::Event.eq(*event_id),
        Some(message_type) => Column::Event
            .eq(*event_id)
            .and(Column::Type.eq(message_type.clone())),
    };

    let res: DeleteResult = Entity::delete_many().filter(filter).exec(con).await?;

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

pub async fn get_pending_messages<C>(
    time: &chrono::DateTime<Utc>,
    mut max_messages: i32,
    con: &C,
) -> Result<Vec<MessageBatch>, DbErr>
where
    C: ConnectionTrait + StreamTrait,
{
    let mut stream = batch_query(con, time).stream(con).await?;

    let mut result = Vec::new();
    while let Some(message_batch) = stream.try_next().await? {
        let mut message_batch = message_batch;

        let vacancies = get_vacancies(message_batch.event_id, con).await?.unwrap();
        let collect_users = message_batch.message_type == MessageType::WaitingListPrompt
            && vacancies.is_have_vacancies();

        if collect_users {
            let mut user_stream = users_query(&message_batch, max_messages, con)
                .stream(con)
                .await?;

            while let Some(user) = user_stream.try_next().await? {
                message_batch.recipients.push(user.user);

                max_messages -= 1;
                if max_messages == 0 {
                    result.push(message_batch);
                    return Ok(result);
                }

                if message_batch.message_type == MessageType::WaitingListPrompt {
                    break; // take not more than one at a time
                }
            }
        }

        if message_batch.recipients.len() == 0 {
            message_outbox::Entity::delete_many()
                .filter(message_outbox::Column::Message.eq(message_batch.message_id))
                .exec(con)
                .await?;

            message_sent::Entity::delete_many()
                .filter(message_sent::Column::Message.eq(message_batch.message_id))
                .exec(con)
                .await?;
        }

        result.push(message_batch);
    }

    Ok(result)
}

#[derive(FromQueryResult)]
pub struct MessageBatch {
    pub message_id: i32,
    pub event_id: i32,
    pub sender: String,
    pub message_type: MessageType,
    pub waiting_list: i32,
    pub text: String,
    pub adult_ticket_price: i32,
    pub child_ticket_price: i32,
    pub recipients: Vec<i32>,
}

fn batch_query<C>(conn: &C, time: &chrono::DateTime<Utc>) -> SelectorRaw<SelectModel<MessageBatch>>
where
    C: ConnectionTrait,
{
    MessageBatch::find_by_statement(Statement::from_sql_and_values(
        conn.get_database_backend(),
        r#"SELECT
                        m.*,
                        m.type as message_type,
                        o.send_at,
                        e.adult_ticket_price,
                        e.child_ticket_price
                    FROM message_outbox as o
                        JOIN messages as m ON o.message = m.id
                        JOIN events as e ON m.event = e.id
                    WHERE o.send_at < $1"#,
        [time.naive_utc().into()],
    ))
}

#[derive(FromQueryResult)]
struct User {
    user: i32,
    sent: String,
}

fn users_query<C>(
    message_batch: &MessageBatch,
    max_messages: i32,
    conn: &C,
) -> SelectorRaw<SelectModel<User>>
where
    C: ConnectionTrait,
{
    User::find_by_statement(
        Statement::from_sql_and_values(
            conn.get_database_backend(),
            "SELECT r.user, s.message as sent FROM \
                        (select user, ts from reservations WHERE event = $1 AND waiting_list = $2 GROUP BY user) as r
                        LEFT JOIN (select user, message from message_sent where message = $3) as s
                        ON r.user = s.user
                        WHERE sent is null ORDER BY r.ts LIMIT $4",
            [
                message_batch.event_id.into(),
                message_batch.waiting_list.into(),
                message_batch.message_id.into(),
                max_messages.into(),
            ],
        )
    )
}
