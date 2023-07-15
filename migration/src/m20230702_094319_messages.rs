use crate::m20220101_000001_create_events::Event;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Message::Table)
                    .col(
                        ColumnDef::new(Message::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Message::Event).integer().not_null())
                    .col(ColumnDef::new(Message::Type).integer().not_null())
                    .col(ColumnDef::new(Message::Sender).string().not_null())
                    .col(ColumnDef::new(Message::WaitingList).integer().not_null())
                    .col(ColumnDef::new(Message::Text).text().not_null())
                    .col(ColumnDef::new(Message::Ts).timestamp().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("FK_message_event")
                    .from(Message::Table, Message::Event)
                    .to(Event::Table, Event::Id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Message::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub enum Message {
    Table,
    Id,
    Event,
    Type,
    Sender,
    WaitingList,
    Text,
    Ts,
}
