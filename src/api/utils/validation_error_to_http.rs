use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use std::fmt::{Debug, Display, Formatter};
use validator::ValidationErrors;

#[derive(Debug)]
pub struct ValidationError {
    errors: ValidationErrors,
}

impl From<ValidationErrors> for ValidationError {
    fn from(value: ValidationErrors) -> Self {
        ValidationError { errors: value }
    }
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ValidationError: {}", self.errors)
    }
}

impl ResponseError for ValidationError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::BadRequest().body(serde_json::to_string(&self.errors).unwrap())
    }
}

pub fn validation_error_to_http(err: ValidationErrors) -> ValidationError {
    ValidationError::from(err)
}
