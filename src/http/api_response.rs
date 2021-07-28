use actix_http::body::{AnyBody, BodySize};
use actix_web::body::MessageBody;
use actix_web::http::{header, HeaderMap, HeaderValue, StatusCode};
use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::Serialize;

use super::*;
use crate::errors;

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
        let region = crate::globals::GLOBAL_SERVER_REGION.lock().unwrap();
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

    pub fn error(err: errors::GenericApiError) -> Self {
        Self::new(err.http_status_code, AnyBody::None, None)
    }
}

impl ApiResponse<String> {
    pub fn success_json<T>(data: &T) -> Self
    where
        T: ?Sized + Serialize,
    {
        let body = serde_json::to_string(data).unwrap();
        Self::new(StatusCode::OK, body, Some(mime::APPLICATION_JSON))
    }

    pub fn error_string(err: errors::GenericApiError) -> Self {
        Self::new(err.http_status_code, err.description, None)
    }

    pub fn error_json(err: errors::GenericApiError, req: &HttpRequest) -> Self {
        let request_id = extract_request_id(req);
        let status_code = err.http_status_code;
        let deployment_id = crate::globals::GLOBAL_DEPLOYMENT_ID
            .lock()
            .unwrap()
            .to_owned();
        let err_res =
            errors::ApiErrorResponse::from(err, req.path().to_owned(), request_id, deployment_id);
        Self::new(
            status_code,
            serde_json::to_string(&err_res).unwrap_or_else(|_| "".to_owned()),
            Some(mime::APPLICATION_JSON),
        )
    }
}

impl Responder for ApiResponse {
    #[inline]
    fn respond_to(self, _: &HttpRequest) -> HttpResponse {
        self.res
    }
}
