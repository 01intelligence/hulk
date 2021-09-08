use crate::utils;
use crate::utils::Duration;

pub(super) const JWT_ALGORITHM: &str = "Bearer";

// Default JWT token for web handlers is one day.
const DEFAULT_JWT_EXPIRY: Duration = utils::hours(24);

// Inter-node JWT token expiry is 15 minutes.
const DEFAULT_INTER_NODE_JWT_EXPIRY: Duration = utils::minutes(15);

// URL JWT token expiry is one minute.
const DEFAULT_URL_JWT_EXPIRY: Duration = utils::minutes(1);

pub fn authenticate_node(access_key: String, secret_key: &str) -> anyhow::Result<String> {
    let mut claims = crate::jwt::StandardClaims::new();
    claims.set_access_key(access_key);
    claims.set_expiry(
        utils::now()
            .checked_add_signed(
                utils::ChronoDuration::from_std(DEFAULT_INTER_NODE_JWT_EXPIRY).unwrap(),
            )
            .unwrap(),
    );
    crate::jwt::sign_with_standard_claims(&claims, secret_key)
}
