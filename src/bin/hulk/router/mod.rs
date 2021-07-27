mod api_router;

use std::sync::MutexGuard;
use regex::Regex;

use actix_http::body::Body;
use actix_web::guard::get_host_uri;
use actix_web::{guard, web, App, AppEntry, HttpServer, Scope};
use api_router::*;
use hulk::{globals, object, objectcache};

use super::*;

struct Api {}

impl Api {
    fn object_api() -> MutexGuard<'static, Option<object::ObjectLayer>> {
        object::get_object_layer()
    }

    fn cache_object_api() -> MutexGuard<'static, Option<objectcache::CacheObjectLayer>> {
        objectcache::get_cache_layer()
    }
}

// Configure server http handler.
pub fn configure_server_handler() -> anyhow::Result<App<AppEntry, Body>> {
    let mut app = App::new();

    let mut scopes = Vec::new();
    for domain_name in globals::GLOBAL_DOMAIN_IPS.lock().unwrap().iter() {
        let host_re = regex::Regex::new(&format!(r#"^(.+)\.{}$"#, regex::escape(domain_name)))?;
        let reserved_host = format!("{}.{}", globals::SYSTEM_RESERVED_BUCKET, domain_name);
        let scope = web::scope("/").guard(guard::fn_guard(move |req| {
            if let Some(uri) = get_host_uri(req) {
                if let Some(uri_host) = uri.host() {
                    // Reserve hulk.<namespace>.svc.<cluster_domain> if in Kubernetes.
                    if *is_kubernetes && uri_host == reserved_host {
                        return false;
                    }
                    // Allow <bucket>.<namespace>.svc.<cluster_domain> and extract bucket.
                    if let Some(caps) = host_re.captures(uri_host) {
                        let bucket = caps.get(1).unwrap().as_str();
                        req.extensions_mut().insert(bucket.to_owned());
                        return true;
                    }
                }
            }
            false
        }));
        scopes.push(scope);
    }
    scopes.push(web::scope("/{bucket}"));

    for scope in scopes {
        app = app.service(scope);
    }

    Ok(app)
}
