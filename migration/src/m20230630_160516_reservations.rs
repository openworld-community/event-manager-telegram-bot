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
                    .table(Reservation::Table)
                    .col(
                        ColumnDef::new(Reservation::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Reservation::Event).integer().not_null())
                    .col(ColumnDef::new(Reservation::User).integer().not_null())
                    .col(ColumnDef::new(Reservation::UserName1).string().not_null())
                    .col(ColumnDef::new(Reservation::UserName2).string().not_null())
                    .col(ColumnDef::new(Reservation::Adults).integer().not_null())
                    .col(ColumnDef::new(Reservation::Children).integer().not_null())
                    .col(
                        ColumnDef::new(Reservation::WaitingList)
                            .integer()
                            .default(0)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Reservation::Ts).timestamp().not_null())
                    .col(ColumnDef::new(Reservation::Payment).string().not_null())
                    .col(ColumnDef::new(Reservation::State).integer().default(0))
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("FK_reservation_event")
                    .from(Reservation::Table, (Reservation::Event))
                    .to(Event::Table, (Event::Id))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_reservation_event")
                    .table(Reservation::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Reservation::Table).to_owned())
            .await
    }
}
/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Reservation {
    Table,
    Id,
    Event,
    User,
    UserName1,
    UserName2,
    Adults,
    Children,
    WaitingList,
    Ts,
    Payment,
    State,
}
