use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, QuerySelect, TransactionTrait, TryIntoModel};
use entity::event;
use entity::event::EventState;
use crate::api::controllers::event::types::{OptionalRawEvent, RawEvent};
use crate::api::shared::Pagination;


pub async fn create_event(
    event_to_create: &RawEvent,
    pool: &DatabaseConnection,
) -> Result<event::Model, DbErr> {
    let now = Utc::now().naive_utc();

    let event = event::ActiveModel {
        id: ActiveValue::NotSet,
        name: ActiveValue::Set(event_to_create.name.clone()),
        link: ActiveValue::Set(event_to_create.link.clone()),
        max_adults: ActiveValue::Set(event_to_create.max_adults),
        max_children: ActiveValue::Set(event_to_create.max_children),
        max_adults_per_reservation: ActiveValue::Set(event_to_create.max_adults_per_reservation),
        max_children_per_reservation: ActiveValue::Set(
            event_to_create.max_children_per_reservation,
        ),
        ts: ActiveValue::Set(now),
        remind: ActiveValue::Set(event_to_create.remind.naive_utc()),
        state: ActiveValue::Set(EventState::default()),
        adult_ticket_price: ActiveValue::Set(event_to_create.adult_ticket_price),
        child_ticket_price: ActiveValue::Set(event_to_create.child_ticket_price),
        currency: ActiveValue::Set(event_to_create.currency.clone()),
    };

    let result = event::Entity::insert(event).exec(pool).await?;

    Ok(event::Entity::find_by_id(result.last_insert_id)
        .one(pool)
        .await?
        .unwrap())
}


pub async fn event_list(pagination: &impl Pagination, pool: &DatabaseConnection) -> Result<Vec<event::Model>, DbErr> {
    event::Entity::find()
        .limit(pagination.limit())
        .offset(pagination.offset())
        .all(pool)
        .await
}

pub async fn get_event(id: i32, pool: &DatabaseConnection) -> Result<Option<event::Model>, DbErr> {
    event::Entity::find_by_id(id).one(pool).await
}
