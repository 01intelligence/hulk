use std::future::{ready, Future, Ready};
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::{timeout};

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::Error;
use futures_util::{ready, FutureExt};
use hulk::globals::{GLOBAL_API_CONFIG, GLOBAL_HTTP_STATS};
use tokio::sync::Semaphore;
use hulk::utils::AtomicExt;
use futures_util::future::{Either, LocalBoxFuture};

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
        let fut = self.service.call(req);
        let sem = match self.requests_max_semaphore {
            Some(ref sem) => sem.clone(),
            None => {
                return Either::Left(fut);
            }
        };
        GLOBAL_HTTP_STATS.s3_requests_in_queue.inc();

        let deadline = self.request_deadline;
        let res = async move {
            match timeout(deadline, async move {
                let _permit = sem.acquire();
                GLOBAL_HTTP_STATS.s3_requests_in_queue.dec();
                fut.await
            }).await {
                Ok(res) => res,
                Err(err) => {
                    GLOBAL_HTTP_STATS.s3_requests_in_queue.dec();
                    todo!()
                }
            }
        }.boxed_local();

        Either::Right(res)
    }
}

#[pin_project::pin_project]
pub struct MaxClientsFuture<S: Service<ServiceRequest>, B> {
    #[pin]
    fut: S::Future,
    requests_max_semaphore: Arc<Semaphore>,
    _b: PhantomData<B>,
}

impl<S, B> Future for MaxClientsFuture<S, B>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    type Output = <S::Future as Future>::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let res = ready!(this.fut.poll(cx));

        Poll::Ready(res)
    }
}
