use std::convert::TryInto;
use std::sync::{Arc, RwLock};

use hulk::utils::PathBuf;
use lazy_static::lazy_static;

const DEFAULT_SYSTEM_CONFIG_DIR: &str = ".hulk";
const CERTS_DIR: &str = "certs";
const CERTS_CA_DIR: &str = "CAs";
const PUBLIC_CERT_FILE: &str = "public.crt";
const PRIVATE_KEY_FILE: &str = "private.key";

lazy_static! {
    pub static ref DEFAULT_CERTS_DIR: PathBuf = get_default_certs_dir();
    pub static ref GLOBAL_CERTS_DIR: Arc<RwLock<PathBuf>> =
        Arc::new(RwLock::new(DEFAULT_CERTS_DIR.clone()));
}

pub fn get_public_cert_file() -> PathBuf {
    GLOBAL_CERTS_DIR.read().unwrap().join(PUBLIC_CERT_FILE)
}

pub fn get_private_key_file() -> PathBuf {
    GLOBAL_CERTS_DIR.read().unwrap().join(PRIVATE_KEY_FILE)
}

pub fn get_certs_ca_dir() -> PathBuf {
    GLOBAL_CERTS_DIR.read().unwrap().join(CERTS_CA_DIR)
}

fn get_default_certs_dir() -> PathBuf {
    let home_dir: PathBuf = dirs::home_dir().unwrap_or_default().try_into().unwrap();
    home_dir.join(CERTS_DIR)
}
