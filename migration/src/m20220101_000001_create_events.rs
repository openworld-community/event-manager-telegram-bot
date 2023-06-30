use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Events::Table)
                    .col(
                        ColumnDef::new(Events::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Events::Name).string().not_null())
                    .col(ColumnDef::new(Events::Link).string().not_null())
                    .col(ColumnDef::new(Events::MaxAdults).integer().not_null())
                    .col(ColumnDef::new(Events::MaxChildren).integer().not_null())
                    .col(ColumnDef::new(Events::MaxAdultsPerReservation).integer().not_null())
                    .col(ColumnDef::new(Events::MaxChildrenPerReservation).integer().not_null())
                    .col(ColumnDef::new(Events::Ts).integer().not_null())
                    .col(ColumnDef::new(Events::Remind).integer().not_null())
                    .col(ColumnDef::new(Events::State).integer().not_null())
                    .col(ColumnDef::new(Events::AdultTicketPrice).integer().not_null().default(0))
                    .col(ColumnDef::new(Events::ChildTicketPrice).integer().not_null().default(0))
                    .col(ColumnDef::new(Events::Currency).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Events::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Events {
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
