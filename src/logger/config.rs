use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;
use crate::config::KVS;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
}

lazy_static! {
    pub static ref DEFAULT_KVS: KVS = KVS(vec![]);
    pub static ref DEFAULT_AUDIT_KVS: KVS = KVS(vec![]);
}
