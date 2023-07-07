use crate::api::controllers::event::types::{OptionalRawEvent, RawEvent};
use crate::api::shared::Pagination;
use chrono::Utc;
use entity::event;
use entity::event::EventState;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QuerySelect,
};

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
        event_start_time: ActiveValue::Set(now),
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

pub async fn event_list(
    pagination: &impl Pagination,
    pool: &DatabaseConnection,
) -> Result<Vec<event::Model>, DbErr> {
    event::Entity::find()
        .limit(pagination.limit())
        .offset(pagination.offset())
        .all(pool)
        .await
}

pub async fn get_event<C>(id: &i32, pool: &C) -> Result<Option<event::Model>, DbErr>
where
    C: ConnectionTrait,
{
    event::Entity::find_by_id(id.clone()).one(pool).await
}

pub async fn update_event<C>(
    id: &i32,
    raw_event: OptionalRawEvent,
    poll: &C,
) -> Result<event::Model, DbErr>
where
    C: ConnectionTrait,
{
    let ac = event::ActiveModel {
        id: ActiveValue::Unchanged(id.clone()),
        state: ActiveValue::Unchanged(EventState::Open),
        name: raw_event
            .name
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.clone())),
        link: raw_event
            .link
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.clone())),
        max_adults: raw_event
            .max_adults
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.clone())),
        max_children: raw_event
            .max_children
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.clone())),
        max_adults_per_reservation: raw_event
            .max_adults_per_reservation
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.clone())),
        max_children_per_reservation: raw_event
            .max_children_per_reservation
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.clone())),
        event_start_time: raw_event
            .event_start_time
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.naive_utc())),
        remind: raw_event
            .remind
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.naive_utc())),
        adult_ticket_price: raw_event
            .adult_ticket_price
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.clone())),
        child_ticket_price: raw_event
            .child_ticket_price
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.clone())),
        currency: raw_event
            .currency
            .map_or(ActiveValue::NotSet, |val| ActiveValue::Set(val.clone())),
    };

    ac.update(poll).await
}
