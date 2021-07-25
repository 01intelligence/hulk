use std::collections::HashMap;

use anyhow::bail;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::*;
use crate::event::{self, target};

// Notification target configuration structure, holds
// information about various notification targets.
/*
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub webhook: target::WebhookArgs,
    pub nats: target::NatsArgs,
    pub redis: target::RedisArgs,
    pub mysql: target::MysqlArgs,
    pub elasticsearch: target::ElasticsearchArgs,
}
*/

#[derive(Clone)]
pub struct HttpClient {
    pub client: reqwest::Client,
    pub root_certs: Vec<reqwest::Certificate>,
}

/*
lazy_static! {
    pub static ref DEFAULT_KVS: KVS = KVS(vec![]);
}
*/

pub async fn test_notification_targets(
    cfg: Config,
    client: HttpClient,
    target_ids: Vec<event::TargetId>,
) -> anyhow::Result<()> {
    match register_notification_targets(cfg, client, target_ids, true, true) {
        Ok(targets) => {
            // Close all targets since we are only testing connections.
            for (_, t) in targets.iter() {
                let mut t = t.lock().await;
                t.close().await;
            }
            Ok(())
        }
        Err(err) => Err(err),
    }
}

pub fn get_notification_targets(
    cfg: Config,
    client: HttpClient,
    target_ids: Vec<event::TargetId>,
    test: bool,
) -> anyhow::Result<event::TargetList> {
    register_notification_targets(cfg, client, target_ids, test, false)
}

fn register_notification_targets(
    cfg: Config,
    client: HttpClient,
    target_ids: Vec<event::TargetId>,
    test: bool,
    return_on_target_error: bool,
) -> anyhow::Result<event::TargetList> {
    let target_list = fetch_registered_targets(cfg, client, test, return_on_target_error)?;
    if test {
        // Verify if user is trying to disable already configured
        // notification targets, based on their target IDs.
        for target_id in target_ids {
            if !target_list.contains(&target_id) {
                bail!("unable to disable configured targets '{}'", target_id);
            }
        }
    }
    Ok(target_list)
}

fn fetch_registered_targets(
    cfg: Config,
    client: HttpClient,
    test: bool,
    return_on_target_error: bool,
) -> anyhow::Result<event::TargetList> {
    let mut target_list = event::TargetList::default();
    // TODO
    Ok(target_list)
}

lazy_static! {
    pub static ref DEFAULT_KVS: HashMap<String, KVS> = maplit::hashmap! {};
}
