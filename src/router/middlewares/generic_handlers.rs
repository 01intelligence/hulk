use std::convert::TryInto;
use std::future::{ready, Future, Ready};
use std::sync::Arc;
use std::time::Duration;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::Error;
use actix_web::http::{header, HeaderMap, Method, StatusCode};
use actix_web::HttpRequest;
use futures_util::future::{Either, LocalBoxFuture};
use futures_util::FutureExt;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use validator::HasLen;

use crate::crypto::{self, SseType};
use crate::errors::ApiError;
use crate::globals::{self, Get, GLOBALS};
use crate::http::{
    get_request_auth_type, guess_is_admin_req, guess_is_browser_req, guess_is_health_check_req,
    guess_is_metrics_req, guess_is_rpc_req, ApiResponse, RequestExtensionsContext,
};
use crate::router::request_to_bucket_object;
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

        let (bucket, _) = request_to_bucket_object(request);
        if bucket.as_ref() == globals::SYSTEM_RESERVED_BUCKET
            || bucket.as_ref() == crate::object::SYSTEM_META_BUCKET
        {
            if !guess_is_rpc_req(request)
                && !guess_is_browser_req(request)
                && !guess_is_health_check_req(request)
                && !guess_is_metrics_req(request)
                && !guess_is_admin_req(request)
            {
                let res = ApiResponse::error_xml(ApiError::AllAccessDisabled.to(), request);
                return Either::Right(ready(Err(res.into())));
            }
        }

        if GLOBALS.browser_enabled.get() && guess_is_browser_req(request) {
            let redirect_location = get_redirect_location(request.path());
            if !redirect_location.is_empty() {
                // TODO
                let _ = http::redirect(
                    request,
                    redirect_location.as_ref(),
                    StatusCode::TEMPORARY_REDIRECT,
                );
            }
        }

        if let Some(res) = http::cross_domain_policy(request) {
            // TODO
        }

        if is_header_size_too_large(headers) {
            GLOBALS.http_stats.total_s3_rejected_header.inc();
            let res = ApiResponse::error_xml(ApiError::MetadataTooLarge.to(), request);
            return Either::Right(ready(Err(res.into())));
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

// Fetch redirect location if url_path satisfies certain
// criteria. Some special names are considered to be
// redirectable, this is purely internal function and
// serves only limited purpose on redirect-handler for
// browser requests.
fn get_redirect_location(url_path: &str) -> std::borrow::Cow<'static, str> {
    if [
        globals::SLASH_SEPARATOR,
        "/webrpc",
        "/login",
        "/favicon-16x16.png",
        "/favicon-32x32.png",
        "/favicon-96x96.png",
    ]
    .contains(&url_path)
    {
        return format!("{}{}", globals::SYSTEM_RESERVED_BUCKET_PATH, url_path).into();
    }
    if url_path == globals::SYSTEM_RESERVED_BUCKET_PATH {
        return globals::SYSTEM_RESERVED_BUCKET_PATH_WITH_SLASH.into();
    }
    "".into()
}

// Maximum size for http headers - See: https://docs.aws.amazon.com/AmazonS3/latest/dev/UsingMetadata.html
const MAX_HEADER_SIZE: usize = 8 * 1024;
// Maximum size for user-defined metadata - See: https://docs.aws.amazon.com/AmazonS3/latest/dev/UsingMetadata.html
const MAX_USER_DATA_SIZE: usize = 2 * 1024;

fn is_header_size_too_large(headers: &HeaderMap) -> bool {
    let mut size = 0;
    let mut user_size = 0;
    for key in headers.keys() {
        size += key.as_str().len();
        for v in headers.get_all(key) {
            size += v.len();
        }
        for prefix in http::USER_METADATA_KEY_PREFIXES {
            if key.as_str().starts_with(prefix) {
                user_size += key.as_str().len();
                break;
            }
        }
    }
    user_size > MAX_USER_DATA_SIZE || size > MAX_HEADER_SIZE
}
