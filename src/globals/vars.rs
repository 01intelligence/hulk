use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

use super::*;
use crate::admin::TraceInfo;
use crate::event;
use crate::pubsub::PubSub;
use crate::strset::StringSet;

#[derive(Default)]
pub struct CliContext {
    pub json: bool,
    pub quiet: bool,
    pub anonymous: bool,
    pub address: String,
    pub strict_s3_compatibility: bool,
}

lazy_static! {
    pub static ref GLOBAL_CLI_CONTEXT: Arc<Mutex<CliContext>> = Arc::new(Mutex::new(Default::default()));

    // Indicates if the running hulk server is distributed setup.
    pub static ref GLOBAL_IS_DIST_ERASURE: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    // Indicates if the running hulk server is an erasure-code backend.
    pub static ref GLOBAL_IS_ERASURE: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    // Indicates if the running hulk is in gateway mode.
    pub static ref GLOBAL_IS_GATEWAY: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    // Name of gateway server, e.g S3, GCS, Azure, etc
    pub static ref GLOBAL_GATEWAY_NAME: Arc<Mutex<String>> = Arc::new(Mutex::new("".to_owned()));

    // This flag is set to 'true' by default
    pub static ref GLOBAL_BROWSER_ENABLED: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));

    // This flag is set to 'true' when HULK_UPDATE env is set to 'off'. Default is false.
    pub static ref GLOBAL_INPLACE_UPDATE_DISABLED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    // This flag is set to 'us-east-1' by default
    pub static ref GLOBAL_SERVER_REGION: Arc<Mutex<String>> = Arc::new(Mutex::new(GLOBAL_DEFAULT_REGION.to_owned()));

    // Local server address (in `host:port` format)
    pub static ref GLOBAL_ADDR: Arc<Mutex<String>> = Arc::new(Mutex::new("".to_owned()));
    // Default port, can be changed through command line.
    pub static ref GLOBAL_PORT: Arc<Mutex<String>> = Arc::new(Mutex::new(GLOBAL_DEFAULT_PORT.to_owned()));
    // Holds the host that was passed using --address
    pub static ref GLOBAL_HOST: Arc<Mutex<String>> = Arc::new(Mutex::new("".to_owned()));
    // Holds the possible host endpoint.
    pub static ref GLOBAL_ENDPOINT: Arc<Mutex<String>> = Arc::new(Mutex::new("".to_owned()));

    // IsSSL indicates if the server is configured with SSL.
    pub static ref GLOBAL_IS_TLS: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    pub static ref GLOBAL_TRACE: Arc<PubSub<TraceInfo>> = Arc::new(PubSub::new(4096));

    // pub static ref GLOBAL_HTTP_LISTEN: Arc<PubSub<event::Event>> = Arc::new(PubSub::new(4096));

    pub static ref GLOBAL_DOMAIN_IPS: Arc<Mutex<StringSet>> = Arc::new(Mutex::new(StringSet::new()));
}

pub fn get_url_scheme() -> &'static str {
    if *GLOBAL_IS_TLS.lock().unwrap() {
        "https"
    } else {
        "http"
    }
}
