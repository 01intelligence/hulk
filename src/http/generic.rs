use actix_web::http::header;
use actix_web::HttpRequest;

use super::*;
use crate::globals::{Get, Guard, GLOBALS};

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
