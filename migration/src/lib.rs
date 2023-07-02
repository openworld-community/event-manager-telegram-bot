pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_events;
mod m20230630_160516_reservations;
mod m20230702_083517_attachments;
mod m20230702_090131_black_list;
mod m20230702_091206_presence;
mod m20230702_092352_group_leaders;
mod m20230702_094319_messages;
mod m20230702_102051_message_outbox;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_events::Migration),
            Box::new(m20230630_160516_reservations::Migration),
            Box::new(m20230702_083517_attachments::Migration),
            Box::new(m20230702_090131_black_list::Migration),
            Box::new(m20230702_091206_presence::Migration),
            Box::new(m20230702_092352_group_leaders::Migration),
            Box::new(m20230702_094319_messages::Migration),
            Box::new(m20230702_102051_message_outbox::Migration),
        ]
    }
}
