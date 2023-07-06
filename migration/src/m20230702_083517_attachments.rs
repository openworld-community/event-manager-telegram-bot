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
                    .table(Attachment::Table)
                    .col(
                        ColumnDef::new(Attachment::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Attachment::Event).integer().not_null())
                    .col(ColumnDef::new(Attachment::User).integer().not_null())
                    .col(ColumnDef::new(Attachment::Attachment).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("FK_attachments_event")
                    .from(Attachment::Table, (Attachment::Event))
                    .to(Event::Table, (Event::Id))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("attachments_unique_event_user_idx")
                    .table(Attachment::Table)
                    .col(Attachment::Attachment)
                    .col(Attachment::User)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_attachments_event")
                    .table(Attachment::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Attachment::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Attachment {
    Table,
    Id,
    Event,
    User,
    Attachment,
}
