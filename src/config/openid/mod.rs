use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {}

lazy_static! {
    pub static ref DEFAULT_KVS: KVS = KVS(vec![]);
}
