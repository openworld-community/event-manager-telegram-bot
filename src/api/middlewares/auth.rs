use crate::configuration::config::JwtKeyAlgorithm;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::{header, StatusCode};
use actix_web::{Error as ActixError, HttpResponse, ResponseError};
use jwt_simple::prelude::MACLike;
use jwt_simple::prelude::NoCustomClaims;
use regex::{Captures, Match, Regex};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use thiserror::Error;

use actix_web::body::{BoxBody, MessageBody};
use actix_web::http::header::ToStrError;

pub struct AuthMiddlewareInner<S> {
    service: Rc<S>,
    key: JwtKeyAlgorithm,
}

#[derive(Debug, Error)]
enum ValidateHeaderError {
    #[error("Missing Authorization header")]
    AuthHeaderExpected,
    #[error("Failed to pars header value")]
    FailedToParsHeaderValue(#[from] ToStrError),
    #[error("Invalid header value")]
    InvalidHeaderValue,
    #[error("Invalid token")]
    InvalidToken,
}

impl ResponseError for ValidateHeaderError {
    fn status_code(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

fn validate_header(req: &ServiceRequest, key: &JwtKeyAlgorithm) -> Result<(), ValidateHeaderError> {
    let header = match req.headers().get(header::AUTHORIZATION) {
        Some(val) => Ok(val),
        None => Err(ValidateHeaderError::AuthHeaderExpected),
    }?;

    let header = header.to_str()?;

    let expr = Regex::new(r"(Bearer)\s(?<token>[\w\.-]+)").unwrap();

    let capt_token = match expr.captures(header) {
        None => Err(ValidateHeaderError::InvalidHeaderValue),
        Some(val) => Ok(val),
    }?;

    let token = match capt_token.name("token") {
        None => Err(ValidateHeaderError::InvalidHeaderValue),
        Some(val) => Ok(val.as_str()),
    }?;

    match key.verify_token::<NoCustomClaims>(&token, None) {
        Ok(_) => Ok(()),
        Err(_) => Err(ValidateHeaderError::InvalidToken),
    }
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareInner<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static,
    B: MessageBody,
{
    type Response = ServiceResponse<B>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let key = self.key.clone();
        Box::pin(async move {
            match validate_header(&req, &key) {
                Ok(_) => {
                    let res = service.call(req).await?;
                    Ok(res)
                }
                Err(err) => Err(err.into()),
            }
        })
    }
}

pub struct AuthMiddleware {
    key: JwtKeyAlgorithm,
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError> + 'static,
    S::Future: 'static,
    B: MessageBody,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type InitError = ();
    type Transform = AuthMiddlewareInner<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(AuthMiddlewareInner {
            service: Rc::new(service),
            key: self.key.clone(),
        }))
    }
}

pub fn auth_middleware(key: &JwtKeyAlgorithm) -> AuthMiddleware {
    AuthMiddleware { key: key.clone() }
}
