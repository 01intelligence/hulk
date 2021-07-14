use super::*;

mod config;
mod migrate;
mod versions;

pub use config::*;
pub use migrate::*;
pub use versions::*;

pub struct ConfigSys {}

impl ConfigSys {
    pub fn init(obj_api: &object::ObjectLayer) -> anyhow::Result<()> {
        todo!()
    }
}
