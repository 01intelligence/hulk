use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

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
    pub host: String,
    pub client_port: u16,
    pub peer_port: u16,
    pub strict_s3_compatibility: bool,
}

#[derive(Default)]
pub struct Globals {
    pub cli_context: Arc<RwLock<CliContext>>,

    // Indicates if the running hulk server is distributed setup.
    pub is_dist_erasure: Arc<AtomicBool>,
    // Indicates if the running hulk server is an erasure-code backend.
    pub is_erasure: Arc<AtomicBool>,
    // Indicates if the running hulk is in gateway mode.
    pub is_gateway: Arc<AtomicBool>,

    // Name of gateway server, e.g S3, GCS, Azure, etc
    pub gateway_name: Arc<Mutex<String>>,

    // This flag is set to 'true' by default
    pub browser_enabled: Arc<AtomicBool>,

    // This flag is set to 'true' when HULK_UPDATE env is set to 'off'. Default is false.
    pub inplace_update_disabled: Arc<AtomicBool>,

    // This flag is set to 'us-east-1' by default
    pub server_region: Arc<Mutex<String>>,

    // Local server address (in `host:port` format)
    pub http_addr: Arc<Mutex<String>>,
    pub rpc_addr: Arc<Mutex<String>>,
    // Default port, can be changed through command line.
    pub http_port: Arc<Mutex<String>>,
    pub rpc_port: Arc<Mutex<String>>,
    // Holds the host that was passed using --address
    pub host: Arc<Mutex<String>>,
    // Holds the possible host endpoint.
    pub endpoint: Arc<Mutex<String>>,

    pub api_config: Arc<Mutex<ApiConfig>>,

    pub storage_class: Arc<Mutex<crate::config::storageclass::Config>>,

    // IsSSL indicates if the server is configured with SSL.
    pub is_tls: Arc<AtomicBool>,

    pub trace: Arc<PubSub<TraceInfo>>,

    pub http_stats: Arc<HttpStats>,

    pub endpoints: Arc<RwLock<crate::endpoint::EndpointServerPools>>,

    pub local_node_name: Arc<Mutex<String>>,

    pub active_cred: Arc<RwLock<crate::auth::Credentials>>,

    // Root domains for virtual host style requests.
    pub domain_names: Arc<RwLock<Vec<String>>>,
    // Root domain IP addresses.
    pub domain_ips: Arc<Mutex<StringSet>>,

    // Deployment ID, unique per deployment.
    pub deployment_id: Arc<RwLock<String>>,

    // If writes to FS backend should be O_SYNC.
    pub fs_osync: Arc<AtomicBool>,
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

pub trait ReadWriteGuard<T: ?Sized> {
    fn guard(&self) -> RwLockReadGuard<'_, T>;
    fn write_guard(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T: ?Sized> ReadWriteGuard<T> for Arc<RwLock<T>> {
    fn guard(&self) -> RwLockReadGuard<'_, T> {
        self.as_ref().read().unwrap()
    }

    fn write_guard(&self) -> RwLockWriteGuard<'_, T> {
        self.as_ref().write().unwrap()
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

impl Get<bool> for Arc<AtomicBool> {
    fn get(&self) -> bool {
        self.load(Ordering::Relaxed)
    }
}

pub trait Set<T> {
    fn set(&self, val: T);
}

impl Set<bool> for Arc<AtomicBool> {
    fn set(&self, val: bool) {
        self.store(val, Ordering::Relaxed);
    }
}

pub fn get_url_scheme() -> &'static str {
    if GLOBALS.is_tls.get() {
        "https"
    } else {
        "http"
    }
}
