use crate::api::utils::ValidationError;
use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, Responder, ResponseError};
use sea_orm::DbErr;
use std::fmt::{Display, Formatter};
use thiserror::Error;
use crate::api::shared::ValidationError;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    DatabaseError(#[from] DbErr),
    #[error("{0}")]
    ValidationError(#[from] ValidationError),
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            AppError::DatabaseError(_) => HttpResponse::new(self.status_code()),
            AppError::ValidationError(err) => err.error_response(),
        }
    }
}
