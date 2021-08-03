use std::convert::TryInto;
use std::future::{ready, Future, Ready};
use std::sync::Arc;
use std::time::Duration;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::Error;
use actix_web::http::{header, HeaderMap, Method};
use actix_web::HttpRequest;
use futures_util::future::{Either, LocalBoxFuture};
use futures_util::FutureExt;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::crypto::{self, SseType};
use crate::errors::ApiError;
use crate::globals::{self, Get, GLOBALS};
use crate::http::{get_request_auth_type, ApiResponse, RequestExtensionsContext};
use crate::utils::{AtomicExt, DateTimeExt};
use crate::{errors, http, utils};

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
    type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let request = req.request();
        let headers = req.headers();

        if contains_reserved_metadata(headers) {
            let res = ApiResponse::error_xml(ApiError::UnsupportedMetadata.to(), request);
            return Either::Right(ready(Err(res.into())));
        }

        if !GLOBALS.is_tls.get()
            && (crypto::SSEC.is_requested(headers) || crypto::SSEC_COPY.is_requested(headers))
        {
            let res = if req.method() == Method::HEAD {
                ApiResponse::error(ApiError::InsecureSSECustomerRequest.to())
            } else {
                ApiResponse::error_xml(ApiError::InsecureSSECustomerRequest.to(), request)
            };
            return Either::Right(ready(Err(res.into())));
        }

        use crate::http::AuthType;
        let auth_type = http::get_request_auth_type(request);
        match auth_type {
            auth_type if http::is_supported_s3_auth_type(auth_type) => {
                // Let top level caller validate for anonymous and known signed requests.
            }
            AuthType::Jwt => {
                // Validate Authorization header if its valid for JWT request.
                // TODO
            }
            AuthType::Sts => {
                // Nothing.
            }
            _ => {
                GLOBALS.http_stats.total_s3_rejected_auth.inc();
                let res =
                    ApiResponse::error_xml(ApiError::SignatureVersionNotSupported.to(), request);
                return Either::Right(ready(Err(res.into())));
            }
        }

        if matches!(
            auth_type,
            AuthType::Signed | AuthType::SignedV2 | AuthType::StreamingSigned
        ) {
            match parse_amz_date_header(request) {
                Err(err) => {
                    GLOBALS.http_stats.total_s3_rejected_time.inc();
                    let res = ApiResponse::error_xml(err.to(), request);
                    return Either::Right(ready(Err(res.into())));
                }
                Ok(amz_date) => {
                    if utils::now().duration_offset(amz_date) > globals::GLOBAL_MAX_SKEW_TIME {
                        GLOBALS.http_stats.total_s3_rejected_time.inc();
                        let res =
                            ApiResponse::error_xml(ApiError::RequestTimeTooSkewed.to(), request);
                        return Either::Right(ready(Err(res.into())));
                    }
                }
            }
        }

        if GLOBALS.browser_enabled.get()
            && req.method() == Method::GET
            && http::guess_is_browser_req(request)
            && req
                .path()
                .starts_with(globals::SYSTEM_RESERVED_BUCKET_PATH_WITH_SLASH)
        {
            let cache_control = if req.path().ends_with(".js")
                || req.path().ends_with(const_format::concatcp!(
                    globals::SYSTEM_RESERVED_BUCKET_PATH,
                    "/favicon.ico",
                )) {
                "max-age=31536000"
            } else {
                "no-store"
            };
            let _ = request
                .special_headers_mut()
                .insert(header::CACHE_CONTROL, cache_control.try_into().unwrap());
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

fn parse_amz_date_header(req: &HttpRequest) -> Result<utils::DateTime, ApiError> {
    for header_name in AMZ_DATE_HEADERS {
        match req.headers().get(header_name) {
            Some(date) => match date.to_str() {
                Ok(date) if !date.is_empty() => return parse_amz_date(date),
                _ => {}
            },
            _ => {}
        }
    }
    Err(ApiError::MissingDateHeader)
}

fn parse_amz_date(amz_data: &str) -> Result<utils::DateTime, ApiError> {
    for fmt in AMZ_DATE_FORMATS {
        match chrono::DateTime::parse_from_str(amz_data, fmt) {
            Ok(d) => return Ok(d.with_timezone(&chrono::Utc)),
            Err(_) => {}
        }
    }
    Err(ApiError::MalformedDate)
}

const AMZ_DATE_FORMATS: [&str; 3] = [
    "%a, %d %b %Y %H:%M:%S %Z",
    "%a, %d %b %Y %H:%M:%S %z",
    http::ISO_8601_FORMAT,
];

const AMZ_DATE_HEADERS: [&str; 2] = ["x-amz-date", "date"];
