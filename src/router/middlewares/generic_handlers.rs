use std::future::{ready, Future, Ready};
use std::sync::Arc;
use std::time::Duration;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::Error;
use actix_web::http::{HeaderMap, Method};
use futures_util::future::{Either, LocalBoxFuture};
use futures_util::FutureExt;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::crypto::{self, SseType};
use crate::errors::ApiError;
use crate::globals::{self, Get, GLOBALS};
use crate::http::ApiResponse;
use crate::utils::AtomicExt;
use crate::{errors, http};

pub struct GenericHandlers {}

impl<S, B> Transform<S, ServiceRequest> for GenericHandlers
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = GenericHandlersMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(GenericHandlersMiddleware { service }))
    }
}

pub struct GenericHandlersMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for GenericHandlersMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<S::Future, http::ApiResponseErrorFuture<ServiceResponse<B>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let headers = req.headers();

        if contains_reserved_metadata(headers) {
            let res = ApiResponse::error_xml(ApiError::UnsupportedMetadata.to(), req.request());
            return Either::Right(res.into());
        }

        if !GLOBALS.is_tls.get()
            && (crypto::SSEC.is_requested(headers) || crypto::SSEC_COPY.is_requested(headers))
        {
            let res = if req.method() == Method::HEAD {
                ApiResponse::error(ApiError::InsecureSSECustomerRequest.to())
            } else {
                ApiResponse::error_xml(ApiError::InsecureSSECustomerRequest.to(), req.request())
            };
            return Either::Right(res.into());
        }

        Either::Left(self.service.call(req))
    }
}

fn contains_reserved_metadata(headers: &HeaderMap) -> bool {
    for k in headers.keys() {
        if k.as_str()
            .starts_with(globals::RESERVED_METADATA_PREFIX_LOWER)
        {
            return true;
        }
    }
    false
}
