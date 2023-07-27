use sea_orm::DbErr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppErrors {
    #[error("{0}")]
    DatabaseError(#[from] DbErr),
}
