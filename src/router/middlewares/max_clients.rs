use std::future::{ready, Ready};
use std::sync::Arc;
use std::time::Duration;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::Error;
use futures_util::future::{Either, LocalBoxFuture};
use futures_util::FutureExt;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::globals::GLOBALS;
use crate::{errors, http};

pub struct MaxClients {
    requests_max_semaphore: Option<Arc<Semaphore>>,
    request_deadline: Duration,
}

impl MaxClients {
    pub fn new(requests_max: usize, request_deadline: Duration) -> Self {
        let semaphore = if requests_max > 0 {
            Some(Arc::new(Semaphore::new(requests_max)))
        } else {
            None
        };
        MaxClients {
            requests_max_semaphore: semaphore,
            request_deadline,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for MaxClients
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = MaxClientsMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(MaxClientsMiddleware {
            service,
            requests_max_semaphore: self.requests_max_semaphore.clone(),
            request_deadline: self.request_deadline,
        }))
    }
}

pub struct MaxClientsMiddleware<S> {
    service: S,
    requests_max_semaphore: Option<Arc<Semaphore>>,
    request_deadline: Duration,
}

impl<S, B> Service<ServiceRequest> for MaxClientsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<S::Future, LocalBoxFuture<'static, Result<ServiceResponse<B>, Error>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let sem = match self.requests_max_semaphore {
            Some(ref sem) => sem.clone(),
            None => {
                return Either::Left(self.service.call(req));
            }
        };
        let _guard = GLOBALS.http_stats.add_requests_in_queue();

        let deadline = self.request_deadline;
        let request = req.request().clone();
        let fut = self.service.call(req);
        let res = async move {
            match timeout(deadline, async move {
                let _permit = sem.acquire().await.unwrap();
                fut.await
            })
            .await
            {
                Ok(res) => res,
                Err(_) => {
                    let res = http::ApiResponse::error_xml(
                        errors::ApiError::OperationMaxedOut.to(),
                        &request,
                    );

                    Err(res.into())
                }
            }
        }
        .boxed_local();

        Either::Right(res)
    }
}
