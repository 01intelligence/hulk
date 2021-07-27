use std::cell::RefCell;
use std::rc::Rc;

use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::{Error, Result};
use futures::future::{FutureExt, LocalBoxFuture};
use hulk::admin::TraceInfo;
use hulk::globals;

pub struct TraceAll {}

impl<S, B> Transform<S, ServiceRequest> for TraceAll
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = TraceAllMiddleware<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(TraceAllMiddleware {
            service: Rc::new(RefCell::new(service)),
        }))
    }
}

pub struct TraceAllMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service<ServiceRequest> for TraceAllMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::Either<
        S::Future,
        LocalBoxFuture<'static, Result<ServiceResponse<B>, Error>>,
    >;

    fn poll_ready(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);
        if globals::GLOBAL_TRACE.subscribers_num() == 0 {
            return futures::future::Either::Left(fut);
        }
        let res = async move {
            let res: Result<S::Response, S::Error> = fut.await;
            globals::GLOBAL_TRACE.publish(TraceInfo {});
            res
        }
        .boxed_local();
        futures::future::Either::Right(res)
    }
}
