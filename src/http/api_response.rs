use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use actix_http::body::{AnyBody, BodySize};
use actix_web::body::MessageBody;
use actix_web::error::Error;
use actix_web::http::{header, HeaderName, HeaderValue, StatusCode};
use actix_web::{HttpRequest, HttpResponse, Responder, ResponseError};
use serde::Serialize;

use super::*;
use crate::errors;
use crate::globals::{self, Guard, GLOBALS};

pub struct ApiResponse<B = AnyBody> {
    res: HttpResponse<B>,
}

impl<B: MessageBody> ApiResponse<B> {
    pub fn new(status: StatusCode, body: B, mime: Option<mime::Mime>) -> Self {
        let size = body.size();
        let mut res = HttpResponse::with_body(status, body);
        let headers = res.headers_mut();
        if let Some(mime) = mime {
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref()).unwrap(),
            );
        }
        match size {
            BodySize::Empty => {
                headers.insert(header::CONTENT_LENGTH, HeaderValue::from(0));
            }
            BodySize::Sized(size) => {
                headers.insert(header::CONTENT_LENGTH, HeaderValue::from(size));
            }
            _ => {}
        }
        let mut res = ApiResponse { res };
        res.set_common_headers();
        res
    }

    pub fn set_common_headers(&mut self) {
        let headers = self.res.headers_mut();
        headers.insert(header::SERVER, HeaderValue::from_static("Hulk"));
        let region = GLOBALS.server_region.guard();
        if !region.is_empty() {
            headers.insert(
                AMZ_BUCKET_REGION.clone(),
                HeaderValue::from_str(&*region).unwrap(),
            );
        }
        headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
        // TODO: Remove sensitive information
    }
}

impl ApiResponse<AnyBody> {
    pub fn no_content() -> Self {
        Self::new(StatusCode::NO_CONTENT, AnyBody::None, None)
    }

    pub fn redirect_see_other() -> Self {
        Self::new(StatusCode::SEE_OTHER, AnyBody::None, None)
    }

    pub fn ok() -> Self {
        Self::new(StatusCode::OK, AnyBody::None, None)
    }

    pub fn success_json<T>(data: &T) -> Self
    where
        T: ?Sized + Serialize,
    {
        let body = serde_json::to_string(data).unwrap();
        Self::new(
            StatusCode::OK,
            AnyBody::from(body),
            Some(mime::APPLICATION_JSON),
        )
    }

    pub fn error(err: errors::GenericApiError) -> ApiResponseError {
        ApiResponseError::new(err.http_status_code, AnyBody::None, None)
    }

    pub fn error_string(err: errors::GenericApiError) -> ApiResponseError {
        ApiResponseError::new(err.http_status_code, AnyBody::from(err.description), None)
    }

    pub fn error_xml(mut err: errors::GenericApiError, req: &HttpRequest) -> ApiResponseError {
        match err.code {
            "InvalidRegion" => {
                err.description = format!(
                    "Region does not match; expecting '{}'.",
                    &*GLOBALS.server_region.guard()
                );
            }
            "AuthorizationHeaderMalformed" => {
                err.description = format!(
                    "The authorization header is malformed; the region is wrong; expecting '{}'.",
                    &*GLOBALS.server_region.guard()
                );
            }
            "AccessDenied" => {
                // The request is from browser and also if browser
                // is enabled we need to redirect.
                if guess_is_browser_req(req) {
                    let mut res = ApiResponseError::new(err.http_status_code, AnyBody::None, None);
                    res.insert_header(
                        header::LOCATION,
                        HeaderValue::from_str(&format!(
                            "{}{}",
                            globals::SYSTEM_RESERVED_BUCKET_PATH,
                            req.path()
                        ))
                        .unwrap(),
                    );
                    return res;
                }
            }
            _ => {}
        }
        let request_id = extract_request_id(req);
        let code = err.code;
        let status_code = err.http_status_code;
        let deployment_id = GLOBALS.deployment_id.guard().to_owned();
        let err_res =
            errors::ApiErrorResponse::from(err, req.path().to_owned(), request_id, deployment_id);
        let body = crate::serde::xml::to_string(&err_res).unwrap_or_else(|_| "".to_owned());
        let mut res = ApiResponseError::new(
            status_code,
            AnyBody::from(body),
            Some(mime::APPLICATION_XML),
        );
        match code {
            "SlowDown"
            | "XMinioServerNotInitialized"
            | "XMinioReadQuorum"
            | "XMinioWriteQuorum" => {
                // Set retry-after header to indicate user-agents to retry request after 120secs.
                // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Retry-After
                res.insert_header(header::RETRY_AFTER, HeaderValue::from_static("120"));
            }
            _ => {}
        }
        res
    }

    pub fn error_json(err: errors::GenericApiError, req: &HttpRequest) -> ApiResponseError {
        let request_id = extract_request_id(req);
        let status_code = err.http_status_code;
        let deployment_id = GLOBALS.deployment_id.guard().to_owned();
        let err_res =
            errors::ApiErrorResponse::from(err, req.path().to_owned(), request_id, deployment_id);
        let body = serde_json::to_string(&err_res).unwrap_or_else(|_| "".to_owned());
        ApiResponseError::new(
            status_code,
            AnyBody::from(body),
            Some(mime::APPLICATION_JSON),
        )
    }

    pub fn response(self) -> HttpResponse {
        self.res
    }
}

impl Responder for ApiResponse {
    #[inline]
    fn respond_to(self, _: &HttpRequest) -> HttpResponse {
        self.res
    }
}

#[derive(Debug)]
pub struct ApiResponseError {
    status: StatusCode,
    body: AnyBody,
    mime: Option<mime::Mime>,
    head_extra: Option<Vec<(HeaderName, HeaderValue, bool)>>,
}

impl ApiResponseError {
    pub fn new(status: StatusCode, body: AnyBody, mime: Option<mime::Mime>) -> Self {
        Self {
            status,
            body,
            mime,
            head_extra: None,
        }
    }

    pub fn append_header(&mut self, key: header::HeaderName, value: HeaderValue) {
        let head = self.head_extra.get_or_insert_default();
        head.push((key, value, false));
    }

    pub fn insert_header(&mut self, key: header::HeaderName, value: HeaderValue) {
        let head = self.head_extra.get_or_insert_default();
        head.push((key, value, true));
    }
}

impl ResponseError for ApiResponseError {
    fn status_code(&self) -> StatusCode {
        self.status
    }
    fn error_response(&self) -> HttpResponse {
        let body = match &self.body {
            AnyBody::None => AnyBody::None,
            AnyBody::Empty => AnyBody::None,
            AnyBody::Bytes(bytes) => AnyBody::Bytes(bytes.clone()),
            AnyBody::Message(_) => panic!("ApiResponseError do not support AnyBody::Message"),
        };
        let mut res = ApiResponse::new(self.status, body, self.mime.clone()).response();
        if let Some(head_extra) = &self.head_extra {
            let headers = res.headers_mut();
            for (key, value, is_insert) in head_extra {
                if *is_insert {
                    let _ = headers.insert(key.clone(), value.clone());
                } else {
                    headers.append(key.clone(), value.clone());
                }
            }
        }
        res
    }
}

impl fmt::Display for ApiResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.status.fmt(f)
    }
}

#[pin_project::pin_project]
pub struct ApiResponseErrorFuture<R> {
    #[pin]
    inner: Option<ApiResponseError>,
    _phantom: PhantomData<R>,
}

impl<R> Future for ApiResponseErrorFuture<R> {
    type Output = Result<R, Error>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let err = this.inner.get_mut().take().unwrap();
        Poll::Ready(Err(err.into()))
    }
}

impl<R> From<ApiResponseError> for ApiResponseErrorFuture<R> {
    fn from(e: ApiResponseError) -> Self {
        ApiResponseErrorFuture {
            inner: Some(e),
            _phantom: PhantomData,
        }
    }
}
