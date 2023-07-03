use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Event::Table)
                    .col(
                        ColumnDef::new(Event::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Event::Name).string().not_null())
                    .col(ColumnDef::new(Event::Link).string().not_null())
                    .col(ColumnDef::new(Event::MaxAdults).integer().not_null())
                    .col(ColumnDef::new(Event::MaxChildren).integer().not_null())
                    .col(
                        ColumnDef::new(Event::MaxAdultsPerReservation)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Event::MaxChildrenPerReservation)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Event::Ts).timestamp().not_null())
                    .col(ColumnDef::new(Event::Remind).integer().not_null())
                    .col(ColumnDef::new(Event::State).integer().not_null())
                    .col(
                        ColumnDef::new(Event::AdultTicketPrice)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Event::ChildTicketPrice)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Event::Currency)
                            .string()
                            .not_null()
                            .default("EUR"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Event::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub enum Event {
    Table,
    Id,
    Name,
    Link,
    MaxAdults,
    MaxChildren,
    MaxAdultsPerReservation,
    MaxChildrenPerReservation,
    Ts,
    Remind,
    State,
    AdultTicketPrice,
    ChildTicketPrice,
    Currency,
}
