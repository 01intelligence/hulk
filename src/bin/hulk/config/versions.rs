use hulk::config;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Serialize, Deserialize, Default)]
pub struct ServerConfigV33 {
    pub version: String,
    credential: auth::Credentials,
    region: String,
    worm: config::BoolFlag,
    storageclass: config::storageclass::Config,
    cache: config::cache::Config,
    notify: config::notify::Config,
    logger: log::Config,
    compression: config::compress::Config,
    openid: config::openid::Config,
    policy: Policy,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Policy {
    opa: config::opa::Config,
}
