use crate::api::controllers::event::types::{OptionalRawEvent, RawEvent};
use crate::api::shared::Pagination;
use chrono::Utc;
use entity::event;
use entity::new_types::EventState;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QuerySelect,
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
    raw_event: &OptionalRawEvent,
    poll: &C,
) -> Result<event::Model, DbErr>
where
    C: ConnectionTrait,
{
    let event = get_event(id, poll).await?;

    match event {
        None => Err(DbErr::RecordNotFound(format!(
            "Event not found with id {}",
            id
        ))),
        Some(current_event) => {
            let mut ac = current_event.into_active_model();

            ac.name = match &raw_event.name {
                Some(name) => ActiveValue::Set(name.clone()),
                _ => ac.name,
            };

            ac.link = match &raw_event.link {
                Some(link) => ActiveValue::Set(link.clone()),
                _ => ac.link,
            };

            ac.max_adults = match &raw_event.max_adults {
                Some(max_adults) => ActiveValue::Set(*max_adults),
                _ => ac.max_adults,
            };

            ac.max_children = match &raw_event.max_children {
                Some(max_children) => ActiveValue::Set(*max_children),
                _ => ac.max_children,
            };

            ac.max_adults_per_reservation = match &raw_event.max_adults_per_reservation {
                Some(max_adults_per_reservation) => ActiveValue::Set(*max_adults_per_reservation),
                _ => ac.max_adults_per_reservation,
            };

            ac.max_children_per_reservation = match &raw_event.max_children_per_reservation {
                Some(max_children_per_reservation) => {
                    ActiveValue::Set(*max_children_per_reservation)
                }
                _ => ac.max_children_per_reservation,
            };

            ac.event_start_time = match &raw_event.event_start_time {
                Some(event_start_time) => ActiveValue::Set(event_start_time.naive_utc()),
                _ => ac.event_start_time,
            };

            ac.remind = match &raw_event.remind {
                Some(remind) => ActiveValue::Set(remind.naive_utc()),
                _ => ac.remind,
            };

            ac.adult_ticket_price = match &raw_event.adult_ticket_price {
                Some(adult_ticket_price) => ActiveValue::Set(*adult_ticket_price),
                _ => ac.adult_ticket_price,
            };

            ac.child_ticket_price = match &raw_event.child_ticket_price {
                Some(child_ticket_price) => ActiveValue::Set(*child_ticket_price),
                _ => ac.child_ticket_price,
            };

            ac.currency = match &raw_event.currency {
                Some(currency) => ActiveValue::Set(currency.clone()),
                _ => ac.currency,
            };

            ac.update(poll).await
        }
    }
}
