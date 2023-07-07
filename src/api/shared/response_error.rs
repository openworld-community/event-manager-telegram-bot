use crate::api::shared::ValidationError;
use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use sea_orm::DbErr;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    DatabaseError(#[from] DbErr),
    #[error("{0}")]
    ValidationError(#[from] ValidationError),
    #[error("Entity not found")]
    NotFoundError,
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFoundError => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR
        }
    }
    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            AppError::ValidationError(err) => err.error_response(),
            _ => HttpResponse::new(self.status_code())
        }
    }
}
