pub const VALUE_SEPARATOR: char = ',';

// Top level common ENVs
pub const ENV_ACCESS_KEY: &str = "HULK_ACCESS_KEY";
pub const ENV_SECRET_KEY: &str = "HULK_SECRET_KEY";
pub const ENV_ROOT_USER: &str = "HULK_ROOT_USER";
pub const ENV_ROOT_PASSWORD: &str = "HULK_ROOT_PASSWORD";

pub const ENV_BROWSER: &str = "HULK_BROWSER";
pub const ENV_DOMAIN: &str = "HULK_DOMAIN";
pub const ENV_REGION_NAME: &str = "HULK_REGION_NAME";
pub const ENV_PUBLIC_IPS: &str = "HULK_PUBLIC_IPS";
pub const ENV_FS_OSYNC: &str = "HULK_FS_OSYNC";
pub const ENV_ARGS: &str = "HULK_ARGS";
pub const ENV_DNS_WEBHOOK: &str = "HULK_DNS_WEBHOOK_ENDPOINT";

pub const ENV_ROOT_DISK_THRESHOLD_SIZE: &str = "HULK_ROOTDISK_THRESHOLD_SIZE";

pub const ENV_UPDATE: &str = "HULK_UPDATE";

pub const ENV_KMS_SECRET_KEY: &str = "HULK_KMS_SECRET_KEY";
pub const ENV_KES_ENDPOINT: &str = "HULK_KMS_KES_ENDPOINT";
pub const ENV_KES_KEY_NAME: &str = "HULK_KMS_KES_KEY_NAME";
pub const ENV_KES_CLIENT_KEY: &str = "HULK_KMS_KES_KEY_FILE";
pub const ENV_KES_CLIENT_CERT: &str = "HULK_KMS_KES_CERT_FILE";
pub const ENV_KES_SERVER_CA: &str = "HULK_KMS_KES_CAPATH";

pub const MAX_CONFIG_JSON_SIZE: usize = 262272;
