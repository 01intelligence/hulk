use std::future::{ready, Ready};

use actix_web::dev::{
    forward_ready, MessageBody, Service, ServiceRequest, ServiceResponse, Transform,
};
use actix_web::error::Error;
use actix_web::{web, FromRequest};
use futures_util::future::{Either, FutureExt, LocalBoxFuture};
use serde::Deserialize;

use crate::globals::{Get, Guard, ReadWriteGuard, GLOBALS};
use crate::http::RequestExtensionsContext;

#[derive(Clone)]
pub struct RequestInfo {}

impl<S, B> Transform<S, ServiceRequest> for RequestInfo
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestInfoMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestInfoMiddleware { service }))
    }
}

pub struct RequestInfoMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestInfoMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        #[derive(Deserialize)]
        struct PathVars {
            bucket: String,
            #[serde(default)]
            object: String,
            #[serde(default)]
            prefix: String,
        }

        let req_parts = req.parts_mut();
        let path_vars = match tokio::runtime::Handle::current()
            .block_on(async { web::Path::<PathVars>::from_request(req_parts.0, req_parts.1).await })
        {
            Ok(path_vars) => path_vars,
            Err(err) => return Either::Right(ready(Err(err.into()))),
        };
        let bucket = path_vars.bucket.clone();
        // TODO: escape
        let object = if path_vars.prefix.is_empty() {
            path_vars.object.clone()
        } else {
            path_vars.prefix.clone()
        };

        let mut ctx = req.request().ctx_mut();
        ctx.request_info = Some(crate::logger::ReqInfo {
            remote_host: req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or_default()
                .to_owned(),
            host: if GLOBALS.is_dist_erasure.get() {
                GLOBALS.local_node_name.guard().clone()
            } else {
                req.head().uri.host().unwrap_or_default().to_owned()
            },
            user_agent: req
                .headers()
                .get(http::header::USER_AGENT)
                .map(|v| v.to_str().unwrap_or_default())
                .unwrap_or_default()
                .to_owned(),
            deployment_id: GLOBALS.deployment_id.guard().clone(),
            request_id: crate::http::extract_request_id(req.request()),
            bucket_name: bucket,
            object_name: object,
            ..Default::default()
        });
        drop(ctx);

        Either::Left(self.service.call(req))
    }
}
