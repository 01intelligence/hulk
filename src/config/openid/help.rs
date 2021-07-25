use lazy_static::lazy_static;

use super::*;
use crate::config::{self, HelpKV, HelpKVS};

lazy_static! {
    pub static ref HELP: HelpKVS = HelpKVS(vec![]);
}
