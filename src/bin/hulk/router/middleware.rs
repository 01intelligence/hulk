use std::str::FromStr;

use actix_cors::Cors;
use actix_web::http::{header, Method};
use hulk::{http, wildcard};

use super::*;

// CORS (Cross Origin Resource Sharing) middleware.
pub fn cors() -> Cors {
    let common_s3_headers = {
        use header::*;
        vec![
            DATE,
            ETAG,
            SERVER,
            CONNECTION,
            ACCEPT_RANGES,
            CONTENT_RANGE,
            CONTENT_ENCODING,
            CONTENT_LENGTH,
            CONTENT_TYPE,
            CONTENT_DISPOSITION,
            LAST_MODIFIED,
            CONTENT_LANGUAGE,
            CACHE_CONTROL,
            RETRY_AFTER,
            EXPIRES,
            http::AMZ_BUCKET_REGION.clone(),
            HeaderName::from_str("X-Amz*").unwrap(),
            HeaderName::from_str("x-amz*").unwrap(),
            HeaderName::from_str("*").unwrap(),
        ]
    };
    Cors::default()
        .allowed_origin_fn(|origin, head| {
            for allowed_origin in &GLOBAL_API_CONFIG.lock().unwrap().cors_allow_origins {
                if let Ok(origin) = origin.to_str() {
                    if wildcard::match_wildcard_simple(allowed_origin, origin) {
                        return true;
                    }
                }
            }
            false
        })
        .allowed_methods(vec![
            Method::GET,
            Method::PUT,
            Method::HEAD,
            Method::POST,
            Method::DELETE,
            Method::OPTIONS,
            Method::PATCH,
        ])
        .allowed_headers(common_s3_headers.clone())
        .expose_headers(common_s3_headers)
        .supports_credentials()
}
