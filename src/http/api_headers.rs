use std::str::FromStr;

use actix_web::http::header::HeaderName;
use actix_web::http::HeaderMap;
use actix_web::HttpRequest;
use lazy_static::lazy_static;

use super::*;

lazy_static! {
    pub static ref AMZ_BUCKET_REGION: HeaderName =
        HeaderName::from_str("X-Amz-Bucket-Region").unwrap();
    pub static ref AMZ_REQUEST_ID: HeaderName = HeaderName::from_str("x-amz-request-id").unwrap();
}

pub fn extract_request_id(req: &HttpRequest) -> String {
    req.ctx()
        .special_headers
        .as_ref()
        .map(|headers| headers.get(AMZ_REQUEST_ID.clone()))
        .flatten()
        .map_or_else(|| "".to_owned(), |h| h.to_str().unwrap().to_owned())
}
