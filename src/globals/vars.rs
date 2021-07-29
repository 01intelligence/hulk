use std::sync::{Arc, Mutex, MutexGuard};

use lazy_static::lazy_static;

use super::*;
use crate::admin::TraceInfo;
use crate::event;
use crate::http::HttpStats;
use crate::pubsub::PubSub;
use crate::router::ApiConfig;
use crate::strset::StringSet;

#[derive(Default)]
pub struct CliContext {
    pub json: bool,
    pub quiet: bool,
    pub anonymous: bool,
    pub address: String,
    pub strict_s3_compatibility: bool,
}

#[derive(Default)]
pub struct Globals {
    pub cli_context: Arc<Mutex<CliContext>>,

    // Indicates if the running hulk server is distributed setup.
    pub is_dist_erasure: Arc<Mutex<bool>>,
    // Indicates if the running hulk server is an erasure-code backend.
    pub is_erasure: Arc<Mutex<bool>>,
    // Indicates if the running hulk is in gateway mode.
    pub is_gateway: Arc<Mutex<bool>>,

    // Name of gateway server, e.g S3, GCS, Azure, etc
    pub gateway_name: Arc<Mutex<String>>,

    // This flag is set to 'true' by default
    pub browser_enabled: Arc<Mutex<bool>>,

    // This flag is set to 'true' when HULK_UPDATE env is set to 'off'. Default is false.
    pub inplace_update_disabled: Arc<Mutex<bool>>,

    // This flag is set to 'us-east-1' by default
    pub server_region: Arc<Mutex<String>>,

    // Local server address (in `host:port` format)
    pub addr: Arc<Mutex<String>>,
    // Default port, can be changed through command line.
    pub port: Arc<Mutex<String>>,
    // Holds the host that was passed using --address
    pub host: Arc<Mutex<String>>,
    // Holds the possible host endpoint.
    pub endpoint: Arc<Mutex<String>>,

    pub api_config: Arc<Mutex<ApiConfig>>,

    // IsSSL indicates if the server is configured with SSL.
    pub is_tls: Arc<Mutex<bool>>,

    pub trace: Arc<PubSub<TraceInfo>>,

    // pub static ref GLOBAL_HTTP_LISTEN: Arc<PubSub<event::Event>>,
    pub http_stats: Arc<HttpStats>,

    pub domain_ips: Arc<Mutex<StringSet>>,

    // Deployment ID, unique per deployment.
    pub deployment_id: Arc<Mutex<String>>,
}

lazy_static! {
    pub static ref GLOBALS: Globals = Default::default();
}

pub trait Guard<T: ?Sized> {
    fn guard(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> Guard<T> for Arc<Mutex<T>> {
    fn guard(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap()
    }
}

pub trait Get<T: Copy> {
    fn get(&self) -> T;
}

impl<T: Copy> Get<T> for Arc<Mutex<T>> {
    fn get(&self) -> T {
        *self.guard()
    }
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

    pub static ref GLOBAL_API_CONFIG: Arc<Mutex<ApiConfig>> =
        Arc::new(Mutex::new(Default::default()));

    // IsSSL indicates if the server is configured with SSL.
    pub static ref GLOBAL_IS_TLS: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    pub static ref GLOBAL_TRACE: Arc<PubSub<TraceInfo>> = Arc::new(PubSub::new(4096));

    // pub static ref GLOBAL_HTTP_LISTEN: Arc<PubSub<event::Event>> = Arc::new(PubSub::new(4096));

    pub static ref GLOBAL_HTTP_STATS: Arc<HttpStats> = Arc::new(Default::default());

    pub static ref GLOBAL_DOMAIN_IPS: Arc<Mutex<StringSet>> = Arc::new(Mutex::new(StringSet::new()));

    // Deployment ID, unique per deployment.
    pub static ref GLOBAL_DEPLOYMENT_ID: Arc<Mutex<String>> = Arc::new(Mutex::new(Default::default()));
}

pub fn get_url_scheme() -> &'static str {
    if *GLOBAL_IS_TLS.lock().unwrap() {
        "https"
    } else {
        "http"
    }
}
