use std::sync::{Arc, Mutex};

use errors::AsError;
use lazy_static::lazy_static;

use super::*;

mod config;
mod migrate;

pub use config::*;
pub use migrate::*;

pub struct ConfigSys {}

impl ConfigSys {
    pub async fn init(api: &object::ObjectLayer) -> anyhow::Result<()> {
        let config_not_found = match check_server_config(api).await {
            Err(err) => {
                if is_config_not_found(&err) {
                    true
                } else {
                    return Err(err);
                }
            }
            _ => false,
        };
        if config_not_found {
            //
            let server_config = hulk::config::Config::new();
            let _ = save_server_config(api, &server_config).await?;
            *GLOBAL_SERVER_CONFIG.lock().unwrap() = Some(server_config);
        }

        let server_config = read_server_config(api).await?;
        // TODO: Override any values from ENVs.
        *GLOBAL_SERVER_CONFIG.lock().unwrap() = Some(server_config);
        Ok(())
    }
}

lazy_static! {
    static ref GLOBAL_SERVER_CONFIG: Arc<Mutex<Option<hulk::config::Config>>> =
        Arc::new(Mutex::new(None));
}
