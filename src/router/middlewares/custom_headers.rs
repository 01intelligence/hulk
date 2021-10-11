use actix_web::middleware::DefaultHeaders;

pub fn custom_headers() -> DefaultHeaders {
    DefaultHeaders::new()
        // Prevents against XSS attacks
        .header("X-XSS-Protection", "1; mode=block")
        // Prevent mixed (HTTP / HTTPS content)
        .header("Content-Security-Policy", "block-all-mixed-content")
}
