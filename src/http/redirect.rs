use std::borrow::Cow;
use std::convert::TryInto;

use actix_web::dev::AnyBody;
use actix_web::http::{header, Method, StatusCode};
use actix_web::{HttpRequest, HttpResponse};

// Replies to the request with a redirect to url,
// which may be a path relative to the request path.
//
// The provided code should be in the 3xx range and is usually
// StatusMovedPermanently, StatusFound or StatusSeeOther.
//
// If the Content-Type header has not been set, Redirect sets it
// to "text/html; charset=utf-8" and writes a small HTML body.
// Setting the Content-Type header to any value, including nil,
// disables that behavior.
pub fn redirect(req: &HttpRequest, url: &str, status_code: StatusCode) -> HttpResponse {
    let mut url = Cow::Borrowed(url);
    if let Ok(u) = url.parse::<http::Uri>() {
        // If url was relative, make its path absolute by
        // combining with request path.
        // The client would probably do this for us,
        // but doing it ourselves is more reliable.
        // See RFC 7231, section 7.1.2
        if u.scheme().is_none() && u.host().is_none() {
            let mut old_path = req.path();
            if old_path.is_empty() {
                // should not happen, but avoid a crash if it does
                old_path = "/";
            }

            // No leading http://server
            if url.is_empty() || url.chars().nth(0).unwrap() != '/' {
                // Make relative path absolute
                if let Some(old_dir) = std::path::Path::new(old_path).parent() {
                    url = Cow::Owned(old_dir.to_string_lossy().into_owned() + url.as_ref());
                }
            }

            let mut query = None;
            if let Some(i) = url.find('?') {
                query = Some(url.to_mut().split_off(i));
            }

            // Clean up but preserve trailing slash
            let trailing = url.ends_with('/');
            url = path_clean::clean(&url).into();
            if trailing && !url.ends_with('/') {
                url.to_mut().push_str("/");
            }
            if let Some(query) = query {
                url.to_mut().push_str(&query);
            }
        }
    }

    let mut res = HttpResponse::new(status_code);
    let mut headers = res.headers_mut();

    // RFC 7231 notes that a short HTML body is usually included in
    // the response because older user agents may not understand 301/307.
    // Do it only if the request didn't already have a Content-Type header.
    let had_ct = headers.contains_key(header::CONTENT_TYPE);

    headers.insert(header::LOCATION, url.as_ref().try_into().unwrap());
    if !had_ct && (req.method() == Method::GET || req.method() == Method::HEAD) {
        headers.insert(
            header::CONTENT_TYPE,
            mime::TEXT_HTML_UTF_8.to_string().try_into().unwrap(),
        );
    }

    // Shouldn't send the body for POST or HEAD; that leaves GET.
    if !had_ct && req.method() == Method::GET {
        res = res.set_body(AnyBody::from(format!(
            r#"<a href="{}">{}</a>"#,
            askama_escape::escape(url.as_ref(), askama_escape::Html),
            status_code.canonical_reason().unwrap_or_default()
        )));
    }

    res
}
