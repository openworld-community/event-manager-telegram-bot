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
                    .table(Presence::Table)
                    .col(
                        ColumnDef::new(Presence::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Presence::Event).integer().not_null())
                    .col(ColumnDef::new(Presence::User).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("FK_presence_event")
                    .from(Presence::Table, Presence::Event)
                    .to(Event::Table, Event::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("presence_event_user_uniq_index")
                    .table(Presence::Table)
                    .col(Presence::Event)
                    .col(Presence::User)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Presence::Table).to_owned())
            .await
    }
}

// conn.execute(
//                         "CREATE TABLE presence (
//                             event           INTEGER NOT NULL,
//                             user            INTEGER NOT NULL
//                             )",
//                         [],
//                     )?;
//                     conn.execute("CREATE INDEX presence_event_index ON presence (event)", [])?;
//                     conn.execute("CREATE UNIQUE INDEX presence_event_user_unique_idx ON presence (event, user)", [])?;

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Presence {
    Table,
    Id,
    Event,
    User,
}
