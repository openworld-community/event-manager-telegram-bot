use crate::api::controllers::event::types::{OptionalRawEvent, RawEvent};
use crate::api::services::message::{
    create_message, delete_enqueued_messages, ResultCreateMessage,
};
use crate::api::shared::Pagination;
use chrono::Utc;
use entity::event;
use entity::new_types::{EventState, MessageType};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QuerySelect,
};

#[derive(Debug)]
pub struct ResultCreateEvent {
    pub event: event::Model,
    pub result_create_message: Option<ResultCreateMessage>,
}

pub async fn create_event<C>(
    event_to_create: &RawEvent,
    con: &C,
) -> Result<ResultCreateEvent, DbErr>
where
    C: ConnectionTrait,
{
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

    let result = event.insert(con).await?;

    if result.event_type() != event::EventType::Announcement {
        let text = format!(
            "\nЗдравствуйте!\nНе забудьте, пожалуйста, что вы записались на\n<a href=\"{}\">{}</a>\
            \nНачало: {}\nПожалуйста, вовремя откажитесь от мест, если ваши планы изменились.\n",
            result.link,
            result.name,
            // todo: fix problem with timezone
            result.event_start_time.format("%d.%m %H:%M"),
        );

        let result_create_message = create_message(
            result.id,
            "Bot",
            0,
            MessageType::Reminder,
            &text,
            event_to_create.remind,
            con,
        )
        .await?;

        return Ok(ResultCreateEvent {
            event: result,
            result_create_message: Some(result_create_message),
        });
    }

    return Ok(ResultCreateEvent {
        event: result,
        result_create_message: None,
    });
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

            let updated_event = ac.update(poll).await?;

            delete_enqueued_messages(&updated_event.id, &MessageType::Reminder, poll).await?;

            // todo: Remained that event details was changed

            Ok(updated_event)
        }
    }
}
