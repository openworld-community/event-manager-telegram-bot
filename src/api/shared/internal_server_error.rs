use actix_web::ResponseError;
use deadpool::managed::PoolError;
use std::fmt::Debug;
use thiserror::Error;
use tokio::task::JoinError;
use tokio_postgres::Error as tokioPostgresError;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("GetConnectionError {0}")]
    GetConnectionError(#[from] PoolError<tokioPostgresError>),
    #[error("DatabaseQueryError {0}")]
    DatabaseQueryError(#[from] String),
}

#[derive(Debug, Error)]
pub enum InternalServerError {
    #[error("error with connection pool {0}")]
    ConnectionPoll(#[from] PoolError<tokioPostgresError>),
    #[error("error with database request {0}")]
    QueryError(#[from] QueryError),
    #[error("error with tokio spawn {0}")]
    TokioJoinError(#[from] JoinError),
}

impl From<QueryError> for InternalServerError {
    fn from(value: QueryError) -> Self {
        match value {
            QueryError::GetConnectionError(err) => InternalServerError::ConnectionPoll(err),
            QueryError::DatabaseQueryError(err) => {
                InternalServerError::QueryError(QueryError::DatabaseQueryError(err))
            }
        }
    }
}

impl ResponseError for InternalServerError {}

pub fn into_internal_server_error_response<Error: Into<InternalServerError>>(
    err: Error,
) -> InternalServerError {
    err.into()
}
