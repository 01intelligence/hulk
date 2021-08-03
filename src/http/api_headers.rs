use actix_web::HttpRequest;

use super::*;

pub fn extract_request_id(req: &HttpRequest) -> String {
    req.ctx()
        .special_headers
        .as_ref()
        .map(|headers| headers.get(AMZ_REQUEST_ID))
        .flatten()
        .map_or_else(|| "".to_owned(), |h| h.to_str().unwrap().to_owned())
}
