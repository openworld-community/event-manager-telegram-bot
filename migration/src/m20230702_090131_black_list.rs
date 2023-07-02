use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BlackList::Table)
                    .col(
                        ColumnDef::new(BlackList::User)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(BlackList::UserName1).string().not_null())
                    .col(ColumnDef::new(BlackList::UserName2).string().not_null())
                    .col(ColumnDef::new(BlackList::Ts).timestamp().not_null())
                    .col(ColumnDef::new(BlackList::Reason).text().default(""))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BlackList::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum BlackList {
    Table,
    User,
    UserName1,
    UserName2,
    Ts,
    Reason,
}
