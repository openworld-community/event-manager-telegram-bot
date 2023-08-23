use sea_orm::DbErr;
use teloxide::RequestError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppErrors {
    #[error("{0}")]
    DatabaseError(#[from] DbErr),
    #[error("Error to get bot name: {0}")]
    ErrorToGetBotName(RequestError)
}
