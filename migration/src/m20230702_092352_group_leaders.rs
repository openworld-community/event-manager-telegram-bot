use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GroupLeader::Table)
                    .col(
                        ColumnDef::new(GroupLeader::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GroupLeader::Event).integer().not_null())
                    .col(ColumnDef::new(GroupLeader::User).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("group_leader_event_user_uniq_index")
                    .table(GroupLeader::Table)
                    .col(GroupLeader::Event)
                    .col(GroupLeader::User)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GroupLeader::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum GroupLeader {
    Table,
    Id,
    Event,
    User,
}
