use actix_web::http::{header, Method};
use actix_web::HttpRequest;
use const_format::concatcp;

use super::*;
use crate::globals::{self, Get, Guard, GLOBALS};

/// Returns true if the request is browser.
/// This implementation just validates user-agent and
/// looks for "Mozilla" string. This is no way certifiable
/// way to know if the request really came from a browser
/// since User-Agent's can be arbitrary. But this is just
/// a best effort function
pub fn guess_is_browser_req(req: &HttpRequest) -> bool {
    let auth_type = get_request_auth_type(req);
    if auth_type != AuthType::Jwt && auth_type != AuthType::Anonymous {
        return false;
    }
    if GLOBALS.browser_enabled.get() {
        return false;
    }
    if let Some(user_agent) = req.headers().get(header::USER_AGENT) {
        if let Ok(user_agent) = user_agent.to_str() {
            if user_agent.contains("Mozilla") {
                return true;
            }
        }
    }
    false
}

const HEALTH_CHECK_PATHS: [&str; 4] = {
    use crate::router;
    [
        concatcp!(
            router::HEALTH_CHECK_PATH_PREFIX,
            router::HEALTH_CHECK_LIVENESS_PATH
        ),
        concatcp!(
            router::HEALTH_CHECK_PATH_PREFIX,
            router::HEALTH_CHECK_READINESS_PATH
        ),
        concatcp!(
            router::HEALTH_CHECK_PATH_PREFIX,
            router::HEALTH_CHECK_CLUSTER_PATH
        ),
        concatcp!(
            router::HEALTH_CHECK_PATH_PREFIX,
            router::HEALTH_CHECK_CLUSTER_READ_PATH
        ),
    ]
};

pub fn guess_is_health_check_req(req: &HttpRequest) -> bool {
    matches!(req.method(), &Method::GET | &Method::HEAD)
        && get_request_auth_type(req) == AuthType::Anonymous
        && HEALTH_CHECK_PATHS.contains(&req.path())
}

const METRICS_PATHS: [&str; 2] = {
    use crate::router;
    [
        concatcp!(
            globals::SYSTEM_RESERVED_BUCKET_PATH,
            router::PROMETHEUS_METRICS_V2_CLUSTER_PATH
        ),
        concatcp!(
            globals::SYSTEM_RESERVED_BUCKET_PATH,
            router::PROMETHEUS_METRICS_V2_NODE_PATH
        ),
    ]
};

pub fn guess_is_metrics_req(req: &HttpRequest) -> bool {
    matches!(
        get_request_auth_type(req),
        AuthType::Anonymous | AuthType::Jwt
    ) && METRICS_PATHS.contains(&req.path())
}

pub fn guess_is_rpc_req(req: &HttpRequest) -> bool {
    req.method() == Method::POST
        && req
            .path()
            .starts_with(globals::SYSTEM_RESERVED_BUCKET_PATH_WITH_SLASH)
}

pub fn guess_is_admin_req(req: &HttpRequest) -> bool {
    req.path().starts_with(crate::router::ADMIN_PATH_PREFIX)
}
