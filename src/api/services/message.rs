use crate::api::services::message_outbox::create_message_outbox;
use chrono::Utc;
use entity::message::{ActiveModel, Column, Entity};
use entity::new_types::MessageType;
use entity::{event, message, message_outbox};
use sea_orm::prelude::*;
use sea_orm::{ActiveValue, DeleteResult, FromQueryResult, Iterable, JoinType, QuerySelect};

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

#[derive(FromQueryResult)]
pub struct PendingMessages {
    pub id: u64,
    pub message: String,
    pub send_at: DateTime,
    pub adult_ticket_price: u64,
    pub child_ticket_price: u64,
    pub waiting_list: u64,
}

pub async fn get_pending_messages<C>(limit: u64, con: &C) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let pending_messages = build_pending_messages_query(limit, now)
        .into_model::<PendingMessages>()
        .all(con)
        .await?;

    for _message in pending_messages {}

    Ok(())
}

fn build_pending_messages_query(
    limit: u64,
    time: DateTime,
) -> Select<entity::message_outbox::Entity> {
    message_outbox::Entity::find()
        .select_only()
        .columns(message_outbox::Column::iter())
        .columns([
            event::Column::AdultTicketPrice,
            event::Column::ChildTicketPrice,
        ])
        .columns([message::Column::WaitingList])
        .join(JoinType::LeftJoin, message_outbox::Relation::Message.def())
        .join(JoinType::LeftJoin, message::Relation::Event.def())
        .filter(message_outbox::Column::SendAt.lt(time))
        .limit(limit)
}

#[test]
fn check_query() {
    use sea_orm::{DbBackend, QueryTrait};

    let time = Utc::now().naive_utc();
    let query = build_pending_messages_query(50, time.clone())
        .build(DbBackend::Postgres)
        .to_string();

    let expected_query = format!(
        "SELECT \"message_outbox\".\"id\", \"message_outbox\".\"message\", \"message_outbox\".\"send_at\", \"event\".\"adult_ticket_price\", \"event\".\"child_ticket_price\", \"message\".\"waiting_list\" FROM \"message_outbox\" LEFT JOIN \"message\" ON \"message_outbox\".\"message\" = \"message\".\"id\" LEFT JOIN \"event\" ON \"message\".\"event\" = \"event\".\"id\" WHERE \"message_outbox\".\"send_at\" < \'{}\' LIMIT {}",
        time.format("%Y-%m-%d %H:%M:%S").to_string(),
        50
    );

    assert_eq!(query, expected_query)
}
