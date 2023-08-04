use crate::util::current_date_time;
use entity::message_sent;
use entity::message_sent::Model;
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, DbErr};

pub async fn add_message_sent<C>(message: i32, user: i32, con: &C) -> Result<Model, DbErr>
where
    C: ConnectionTrait,
{
    let ac = message_sent::ActiveModel {
        id: Default::default(),
        message: ActiveValue::Set(message),
        user: ActiveValue::Set(user),
        ts: ActiveValue::Set(current_date_time()),
    };

    ac.insert(con).await
}
