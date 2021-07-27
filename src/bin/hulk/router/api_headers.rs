use std::str::FromStr;

use actix_web::http::header::HeaderName;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref AMZ_BUCKET_REGION: HeaderName =
        HeaderName::from_str("X-Amz-Bucket-Region").unwrap();
}
