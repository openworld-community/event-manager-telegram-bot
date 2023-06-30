pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_events;
mod m20230630_160516_reservations;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_events::Migration),
            Box::new(m20230630_160516_reservations::Migration),
        ]
    }
}
