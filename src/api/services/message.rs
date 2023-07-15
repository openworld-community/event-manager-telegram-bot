use entity::message::{Column, Entity};
use entity::new_types::MessageType;
use sea_orm::prelude::*;
use sea_orm::DeleteResult;

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
