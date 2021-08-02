use std::time::Duration;

use crate::utils;

pub(super) const JWT_ALGORITHM: &str = "Bearer";

// Default JWT token for web handlers is one day.
const DEFAULT_JWT_EXPIRY: Duration = utils::hours(24);

// Inter-node JWT token expiry is 15 minutes.
const DEFAULT_INTER_NODE_JWT_EXPIRY: Duration = utils::minutes(15);

// URL JWT token expiry is one minute.
const DEFAULT_URL_JWT_EXPIRY: Duration = utils::minutes(1);
