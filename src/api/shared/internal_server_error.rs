use actix_web::ResponseError;
use r2d2::Error as r2d2Error;
use rusqlite::Error as queryError;
use std::fmt::Debug;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("GetConnectionError {0}")]
    GetConnectionError(#[from] r2d2::Error),
    #[error("DatabaseQueryError {0}")]
    DatabaseQueryError(#[from] rusqlite::Error),
}

#[derive(Debug, Error)]
pub enum InternalServerError {
    #[error("error with connection pool {0}")]
    ConnectionPoll(#[from] r2d2Error),
    #[error("error with database request {0}")]
    QueryError(#[from] queryError),
    #[error("error with tokio spawn {0}")]
    TokioJoinError(#[from] JoinError),
}

impl From<QueryError> for InternalServerError {
    fn from(value: QueryError) -> Self {
        match value {
            QueryError::GetConnectionError(err) => InternalServerError::ConnectionPoll(err),
            QueryError::DatabaseQueryError(err) => InternalServerError::QueryError(err),
        }
    }
}

impl ResponseError for InternalServerError {}

pub fn into_internal_server_error_response<Error: Into<InternalServerError>>(
    err: Error,
) -> InternalServerError {
    err.into()
}
