use actix_http::body::{AnyBody, BodySize};
use actix_web::body::MessageBody;
use actix_web::http::{header, HeaderValue, StatusCode};
use actix_web::{HttpRequest, HttpResponse, Responder};

use super::*;

pub struct ApiResponse<B = AnyBody> {
    res: HttpResponse<B>,
}

impl<B: MessageBody> ApiResponse<B> {
    pub fn new(status: StatusCode, body: B) -> Self {
        let size = body.size();
        let mut res = HttpResponse::with_body(status, body);
        let headers = res.headers_mut();
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

impl Responder for ApiResponse {
    #[inline]
    fn respond_to(self, _: &HttpRequest) -> HttpResponse {
        self.res
    }
}
