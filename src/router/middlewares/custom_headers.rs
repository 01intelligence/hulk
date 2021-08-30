use actix_web::middleware::DefaultHeaders;

use crate::{http, utils};

pub fn custom_headers() -> DefaultHeaders {
    DefaultHeaders::new()
        // Prevents against XSS attacks
        .header("X-XSS-Protection", "1; mode=block")
        // Prevent mixed (HTTP / HTTPS content)
        .header("Content-Security-Policy", "block-all-mixed-content")
        // Sets x-amz-request-id header
        .header(
            http::AMZ_REQUEST_ID,
            format!("{:X}", utils::now().timestamp_nanos()),
        )
}
