use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::*;
use crate::event::target;

// Notification target configuration structure, holds
// information about various notification targets.
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub webhook: target::WebhookArgs,
    pub nats: target::NatsArgs,
    pub redis: target::RedisArgs,
    pub mysql: target::MysqlArgs,
    pub elasticsearch: target::ElasticsearchArgs,
}
