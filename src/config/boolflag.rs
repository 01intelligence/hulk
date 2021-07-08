use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum BoolFlag {
    #[serde(rename = "on")]
    On,
    #[serde(rename = "off")]
    Off,
}

impl Default for BoolFlag {
    fn default() -> Self {
        BoolFlag::Off
    }
}

impl fmt::Display for BoolFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            BoolFlag::On => "on",
            BoolFlag::Off => "off",
        };
        write!(f, "{}", s)
    }
}
