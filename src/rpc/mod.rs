mod storage_server;

pub use storage_server::*;
use tonic::{Request, Status};

use crate::globals::{Guard, GLOBALS};
use crate::utils::{self, DateTimeExt, DateTimeFormatExt};

const DEFAULT_SKEW_TIME: utils::Duration = utils::minutes(15);

const NO_AUTH_TOKEN: &str = "JWT token missing";

pub fn validate_request(req: Request<()>) -> Result<Request<()>, Status> {
    let meta = req.metadata();

    let get_str = |key: &str| {
        meta.get(key)
            .and_then(|val| val.to_str().ok())
            .ok_or_else(|| Status::unauthenticated(NO_AUTH_TOKEN))
    };

    let token = get_str("authorization")?;
    let active_cred = GLOBALS.active_cred.guard();
    let claims = crate::jwt::parse_with_standard_claims(token, active_cred.secret_key.as_bytes())
        .map_err(|_| Status::unauthenticated(NO_AUTH_TOKEN))?;

    let owner =
        claims.access_key == active_cred.access_key || claims.subject == active_cred.access_key;
    if !owner {
        return Err(Status::unauthenticated(NO_AUTH_TOKEN));
    }

    // TODO: claims.audience

    let req_time = get_str("X-Hulk-Time")?;
    let req_time = utils::DateTime::from_rfc3339(req_time)
        .map_err(|_| Status::unauthenticated(NO_AUTH_TOKEN))?;
    let delta = utils::now().duration_offset(req_time);
    if delta > DEFAULT_SKEW_TIME {
        return Err(Status::unauthenticated(NO_AUTH_TOKEN));
    }

    Ok(req)
}
