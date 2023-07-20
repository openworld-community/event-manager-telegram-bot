use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::task::{Context, Poll};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use chrono::Utc;
use tracing::info;


pub struct LogTime;

impl LogTime {
    pub fn new() -> LogTime {
        LogTime {}
    }
}


impl<S, B> Transform<S, ServiceRequest> for LogTime
    where
        S: Service<ServiceRequest, Response=ServiceResponse<B>, Error=Error>,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = LogTimeMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LogTimeMiddleware { service }))
    }
}


pub struct LogTimeMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for LogTimeMiddleware<S>
    where
        S: Service<ServiceRequest, Response=ServiceResponse<B>, Error=Error>,
        S::Future: 'static,
        B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;

    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + 'static>>;

    fn poll_ready(&self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Utc::now();
        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await;
            let end_time = Utc::now();

            let request_time = end_time - start_time;

            info!( "request time: {}", request_time);

            res
        })
    }
}
