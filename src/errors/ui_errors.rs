use std::any::Any;
use std::fmt;

use colored::*;
use const_format::formatcp;
use thiserror::Error;

use crate::config::*;

#[derive(Error, Clone, Debug, Default)]
pub struct UiErrorItem {
    msg: String,
    action: String,
    hint: String,
    detail: Option<String>,
}

#[derive(Error, Clone, Debug, Default)]
pub struct UiErrorItemConst {
    msg: &'static str,
    action: &'static str,
    hint: &'static str,
}

impl fmt::Display for UiErrorItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.detail {
            Some(ref detail) => write!(f, "{}", detail),
            None => {
                if !self.msg.is_empty() {
                    write!(f, "{}", self.msg)
                } else {
                    write!(f, "<None>")
                }
            }
        }
    }
}

impl fmt::Display for UiErrorItemConst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.msg.is_empty() {
            write!(f, "{}", self.msg)
        } else {
            write!(f, "<None>")
        }
    }
}

impl UiErrorItem {
    fn new(msg: &str, action: &str, hint: &str) -> UiErrorItem {
        UiErrorItem {
            msg: msg.to_owned(),
            action: action.to_owned(),
            hint: hint.to_owned(),
            detail: None,
        }
    }

    fn from_error<E: std::error::Error + 'static>(err: Option<E>) -> UiErrorItem {
        match err {
            None => UiErrorItem::default(),
            Some(err) => {
                let e = &err as &dyn Any;
                if let Some(e) = e.downcast_ref::<UiErrorItem>() {
                    return e.clone();
                }
                // TODO: downcast to other error types
                return UiErrorItem {
                    msg: err.to_string(),
                    ..Default::default()
                };
            }
        }
    }

    pub fn msg(&self, msg: String) -> UiErrorItem {
        let mut e = self.clone();
        e.msg = msg;
        e
    }

    pub fn hint(&self, hint: String) -> UiErrorItem {
        let mut e = self.clone();
        e.hint = hint;
        e
    }

    pub fn error(&self, err: anyhow::Error) -> UiErrorItem {
        let mut e = self.clone();
        e.detail = Some(err.to_string());
        e
    }

    pub fn format<E: std::error::Error + 'static>(
        intro_msg: &str,
        err: Option<E>,
        json: bool,
    ) -> String {
        let ui_err = Self::from_error(err);
        if json {
            return match ui_err.detail {
                Some(ref detail) => format!("{}: {}", ui_err.msg, detail),
                None => ui_err.msg,
            };
        }

        let mut intro_msg = String::from(intro_msg);
        intro_msg.push_str(": ");
        let msg = if !ui_err.msg.is_empty() {
            (&ui_err.msg as &str).bold()
        } else {
            "<None>".bold()
        };
        intro_msg.push_str(&msg);
        let mut rendered = intro_msg.red().to_string();
        rendered.push_str("\n");
        if !ui_err.action.is_empty() {
            rendered.push_str("> ");
            rendered.push_str(&(&ui_err.action as &str).black().on_yellow());
        }
        if !ui_err.hint.is_empty() {
            rendered.push_str(&"HINT:".bold());
            rendered.push_str("\n  ");
            rendered.push_str(&ui_err.hint);
        }
        rendered
    }
}

impl std::convert::From<&UiErrorItemConst> for UiErrorItem {
    fn from(item: &UiErrorItemConst) -> Self {
        UiErrorItem {
            msg: item.msg.to_string(),
            action: item.action.to_string(),
            hint: item.hint.to_string(),
            detail: None,
        }
    }
}

impl UiErrorItemConst {
    const fn new(msg: &'static str, action: &'static str, hint: &'static str) -> UiErrorItemConst {
        UiErrorItemConst { msg, action, hint }
    }

    pub fn msg(&self, msg: String) -> UiErrorItem {
        let mut e = UiErrorItem::from(self);
        e.msg = msg;
        e
    }
}

#[non_exhaustive]
pub enum UiError {
    InvalidBrowserValue,
    InvalidFSOSyncValue,
    OverlappingDomainValue,
    InvalidDomainValue,
    InvalidErasureSetSize,
    InvalidWormValue,
    InvalidCacheDrivesValue,
    InvalidCacheExcludesValue,
    InvalidCacheExpiryValue,
    InvalidCacheQuota,
    InvalidCacheAfter,
    InvalidCacheWatermarkLow,
    InvalidCacheWatermarkHigh,
    InvalidCacheEncryptionKey,
    InvalidCacheRange,
    InvalidCacheCommitValue,
    InvalidCacheSetting,
    InvalidCredentialsBackendEncrypted,
    InvalidCredentials,
    MissingEnvCredentialRootUser,
    MissingEnvCredentialRootPassword,
    MissingEnvCredentialAccessKey,
    MissingEnvCredentialSecretKey,
    InvalidErasureEndpoints,
    InvalidNumberOfErasureEndpoints,
    StorageClassValue,
    UnexpectedBackendVersion,
    InvalidAddressFlag,
    InvalidFSEndpoint,
    UnsupportedBackend,
    UnableToWriteInBackend,
    PortAlreadyInUse,
    PortAccess,
    SSLUnexpectedError,
    SSLUnexpectedData,
    SSLNoPassword,
    NoCertsAndHTTPSEndpoints,
    CertsAndHTTPEndpoints,
    SSLWrongPassword,
    UnexpectedError,
    InvalidCompressionIncludesValue,
    InvalidGWSSEValue,
    InvalidGWSSEEnvValue,
    InvalidReplicationWorkersValue,
}

impl UiError {
    pub fn msg(&self, msg: String) -> UiErrorItem {
        self.value().msg(msg)
    }

    pub fn value(&self) -> &'static UiErrorItemConst {
        match *self {
            UiError::InvalidBrowserValue => &INVALID_BROWSER_VALUE,
            UiError::InvalidFSOSyncValue => &INVALID_FS_OSYNC_VALUE,
            UiError::OverlappingDomainValue => &OVERLAPPING_DOMAIN_VALUE,
            UiError::InvalidDomainValue => &INVALID_DOMAIN_VALUE,
            UiError::InvalidErasureSetSize => &INVALID_ERASURE_SET_SIZE,
            UiError::InvalidWormValue => &INVALID_WORM_VALUE,
            UiError::InvalidCacheDrivesValue => &INVALID_CACHE_DRIVES_VALUE,
            UiError::InvalidCacheExcludesValue => &INVALID_CACHE_EXCLUDES_VALUE,
            UiError::InvalidCacheExpiryValue => &INVALID_CACHE_EXPIRY_VALUE,
            UiError::InvalidCacheQuota => &INVALID_CACHE_QUOTA,
            UiError::InvalidCacheAfter => &INVALID_CACHE_AFTER,
            UiError::InvalidCacheWatermarkLow => &INVALID_CACHE_WATERMARK_LOW,
            UiError::InvalidCacheWatermarkHigh => &INVALID_CACHE_WATERMARK_HIGH,
            UiError::InvalidCacheEncryptionKey => &INVALID_CACHE_ENCRYPTION_KEY,
            UiError::InvalidCacheRange => &INVALID_CACHE_RANGE,
            UiError::InvalidCacheCommitValue => &INVALID_CACHE_COMMIT_VALUE,
            UiError::InvalidCacheSetting => &INVALID_CACHE_SETTING,
            UiError::InvalidCredentialsBackendEncrypted => &INVALID_CREDENTIALS_BACKEND_ENCRYPTED,
            UiError::InvalidCredentials => &INVALID_CREDENTIALS,
            UiError::MissingEnvCredentialRootUser => &MISSING_ENV_CREDENTIAL_ROOT_USER,
            UiError::MissingEnvCredentialRootPassword => &MISSING_ENV_CREDENTIAL_ROOT_PASSWORD,
            UiError::MissingEnvCredentialAccessKey => &MISSING_ENV_CREDENTIAL_ACCESS_KEY,
            UiError::MissingEnvCredentialSecretKey => &MISSING_ENV_CREDENTIAL_SECRET_KEY,
            UiError::InvalidErasureEndpoints => &INVALID_ERASURE_ENDPOINTS,
            UiError::InvalidNumberOfErasureEndpoints => &INVALID_NUMBER_OF_ERASURE_ENDPOINTS,
            UiError::StorageClassValue => &STORAGE_CLASS_VALUE,
            UiError::UnexpectedBackendVersion => &UNEXPECTED_BACKEND_VERSION,
            UiError::InvalidAddressFlag => &INVALID_ADDRESS_FLAG,
            UiError::InvalidFSEndpoint => &INVALID_FS_ENDPOINT,
            UiError::UnsupportedBackend => &UNSUPPORTED_BACKEND,
            UiError::UnableToWriteInBackend => &UNABLE_TO_WRITE_IN_BACKEND,
            UiError::PortAlreadyInUse => &PORT_ALREADY_IN_USE,
            UiError::PortAccess => &PORT_ACCESS,
            UiError::SSLUnexpectedError => &SSL_UNEXPECTED_ERROR,
            UiError::SSLUnexpectedData => &SSL_UNEXPECTED_DATA,
            UiError::SSLNoPassword => &SSL_NO_PASSWORD,
            UiError::NoCertsAndHTTPSEndpoints => &NO_CERTS_AND_HTTPS_ENDPOINTS,
            UiError::CertsAndHTTPEndpoints => &CERTS_AND_HTTP_ENDPOINTS,
            UiError::SSLWrongPassword => &SSL_WRONG_PASSWORD,
            UiError::UnexpectedError => &UNEXPECTED_ERROR,
            UiError::InvalidCompressionIncludesValue => &INVALID_COMPRESSION_INCLUDES_VALUE,
            UiError::InvalidGWSSEValue => &INVALID_GW_SSE_VALUE,
            UiError::InvalidGWSSEEnvValue => &INVALID_GW_SSE_ENV_VALUE,
            UiError::InvalidReplicationWorkersValue => &INVALID_REPLICATION_WORKERS_VALUE,
        }
    }
}

const INVALID_BROWSER_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid browser value",
    "Please check the passed value",
    "Browser can only accept `on` and `off` values. To disable web browser access, set this value to `off`",
);

const INVALID_FS_OSYNC_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid O_SYNC value",
    "Please check the passed value",
    "Can only accept `on` and `off` values. To enable O_SYNC for fs backend, set this value to `on`",
);

const OVERLAPPING_DOMAIN_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Overlapping domain values",
    "Please check the passed value",
    "HULK_DOMAIN only accepts non-overlapping domain values",
);

const INVALID_DOMAIN_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid domain value",
    "Please check the passed value",
    "Domain can only accept DNS compatible values",
);

const INVALID_ERASURE_SET_SIZE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid erasure set size",
    "Please check the passed value",
    "Erasure set can only accept any of [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] values",
);

const INVALID_WORM_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid WORM value",
    "Please check the passed value",
    "WORM can only accept `on` and `off` values. To enable WORM, set this value to `on`",
);

const INVALID_CACHE_DRIVES_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid cache drive value",
    "Please check the value in this ENV variable",
    "HULK_CACHE_DRIVES: Mounted drives or directories are delimited by `,`",
);

const INVALID_CACHE_EXCLUDES_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid cache excludes value",
    "Please check the passed value",
    "HULK_CACHE_EXCLUDE: Cache exclusion patterns are delimited by `,`",
);

const INVALID_CACHE_EXPIRY_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid cache expiry value",
    "Please check the passed value",
    "HULK_CACHE_EXPIRY: Valid cache expiry duration must be in days",
);

const INVALID_CACHE_QUOTA: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid cache quota value",
    "Please check the passed value",
    "HULK_CACHE_QUOTA: Valid cache quota value must be between 0-100",
);

const INVALID_CACHE_AFTER: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid cache after value",
    "Please check the passed value",
    "HULK_CACHE_AFTER: Valid cache after value must be 0 or greater",
);

const INVALID_CACHE_WATERMARK_LOW: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid cache low watermark value",
    "Please check the passed value",
    "HULK_CACHE_WATERMARK_LOW: Valid cache low watermark value must be between 0-100",
);

const INVALID_CACHE_WATERMARK_HIGH: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid cache high watermark value",
    "Please check the passed value",
    "HULK_CACHE_WATERMARK_HIGH: Valid cache high watermark value must be between 0-100",
);

const INVALID_CACHE_ENCRYPTION_KEY: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid cache encryption master key value",
    "Please check the passed value",
    "HULK_CACHE_ENCRYPTION_MASTER_KEY: For more information, please refer to",
);

const INVALID_CACHE_RANGE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid cache range value",
    "Please check the passed value",
    "HULK_CACHE_RANGE: Valid expected value is `on` or `off`",
);

const INVALID_CACHE_COMMIT_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid cache commit value",
    "Please check the passed value",
    "HULK_CACHE_COMMIT: Valid expected value is `writeback` or `writethrough`",
);

const INVALID_CACHE_SETTING: UiErrorItemConst = UiErrorItemConst::new(
    "Incompatible cache setting",
    "Please check the passed value",
    "HULK_CACHE_AFTER cannot be used with HULK_CACHE_COMMIT setting",
);

const INVALID_CREDENTIALS_BACKEND_ENCRYPTED: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid credentials",
    "Please set correct credentials in the environment for decryption",
    "Detected encrypted config backend, correct access and secret keys should be specified via environment variables HULK_ROOT_USER and HULK_ROOT_PASSWORD to be able to decrypt the hulk config, user IAM and policies",
);

const INVALID_CREDENTIALS: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid credentials",
    "Please provide correct credentials",
    "Access key length should be at least 3, and secret key length at least 8 characters",
);

const MISSING_ENV_CREDENTIAL_ROOT_USER: UiErrorItemConst = UiErrorItemConst::new(
    formatcp!("Missing credential environment variable, \"{}\"", ENV_ROOT_USER),
    formatcp!("Environment variable \"{}\" is missing", ENV_ROOT_USER),
    "Root user name (access key) and root password (secret key) are expected to be specified via environment variables HULK_ROOT_USER and HULK_ROOT_PASSWORD respectively",
);

const MISSING_ENV_CREDENTIAL_ROOT_PASSWORD: UiErrorItemConst = UiErrorItemConst::new(
    formatcp!("Missing credential environment variable, \"{}\"", ENV_ROOT_PASSWORD),
    formatcp!("Environment variable \"{}\" is missing", ENV_ROOT_PASSWORD),
    "Root user name (access key) and root password (secret key) are expected to be specified via environment variables HULK_ROOT_USER and HULK_ROOT_PASSWORD respectively",
);

const MISSING_ENV_CREDENTIAL_ACCESS_KEY: UiErrorItemConst = UiErrorItemConst::new(
    formatcp!("Missing credential environment variable, \"{}\"", ENV_ACCESS_KEY),
    formatcp!("Environment variables \"{}\" and \"{}\" are deprecated", ENV_ACCESS_KEY, ENV_SECRET_KEY),
    "Root user name (access key) and root password (secret key) are expected to be specified via environment variables HULK_ROOT_USER and HULK_ROOT_PASSWORD respectively",
);

const MISSING_ENV_CREDENTIAL_SECRET_KEY: UiErrorItemConst = UiErrorItemConst::new(
    formatcp!("Missing credential environment variable, \"{}\"", ENV_SECRET_KEY),
    formatcp!("Environment variables \"{}\" and \"{}\" are deprecated", ENV_SECRET_KEY, ENV_ACCESS_KEY),
    "Root user name (access key) and root password (secret key) are expected to be specified via environment variables HULK_ROOT_USER and HULK_ROOT_PASSWORD respectively",
);

const INVALID_ERASURE_ENDPOINTS: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid endpoint(s) in erasure mode",
    "Please provide correct combination of local/remote paths",
    "For more information, please refer to",
);

const INVALID_NUMBER_OF_ERASURE_ENDPOINTS: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid total number of endpoints for erasure mode",
    "Please provide an even number of endpoints greater or equal to 4",
    "For more information, please refer to",
);

const STORAGE_CLASS_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid storage class value",
    "Please check the value",
    r#"HULK_STORAGE_CLASS_STANDARD: Format "EC:<Default_Parity_Standard_Class>" (e.g. "EC:3"). This sets the number of parity disks for hulk server in Standard mode. Objects are stored in Standard mode, if storage class is not defined in Put request
HULK_STORAGE_CLASS_RRS: Format "EC:<Default_Parity_Reduced_Redundancy_Class>" (e.g. "EC:3"). This sets the number of parity disks for hulk server in Reduced Redundancy mode. Objects are stored in Reduced Redundancy mode, if Put request specifies RRS storage class
Refer to the link for more information"#,
);

const UNEXPECTED_BACKEND_VERSION: UiErrorItemConst = UiErrorItemConst::new(
    "Backend version seems to be too recent",
    "Please update to the latest hulk version",
    "",
);

const INVALID_ADDRESS_FLAG: UiErrorItemConst = UiErrorItemConst::new(
    "--address input is invalid",
    "Please check --address parameter",
    r"--address binds to a specific ADDRESS:PORT, ADDRESS can be an IPv4/IPv6 address or hostname (default port is ':9000')
	Examples: --address ':443'
		  --address '172.16.34.31:9000'
		  --address '[fe80::da00:a6c8:e3ae:ddd7]:9000'",
);

const INVALID_FS_ENDPOINT: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid endpoint for standalone FS mode",
    "Please check the FS endpoint",
    r"FS mode requires only one writable disk path
Example 1:
   $ hulk server /data/hulk/",
);

const UNSUPPORTED_BACKEND: UiErrorItemConst = UiErrorItemConst::new(
    "Unable to write to the backend",
    "Please ensure your disk supports O_DIRECT",
    "",
);

const UNABLE_TO_WRITE_IN_BACKEND: UiErrorItemConst = UiErrorItemConst::new(
    "Unable to write to the backend",
    "Please ensure hulk binary has write permissions for the backend",
    "Verify if hulk binary is running as the same user who has write permissions for the backend",
);

const PORT_ALREADY_IN_USE: UiErrorItemConst = UiErrorItemConst::new(
    "Port is already in use",
    "Please ensure no other program uses the same address/port",
    "",
);

const PORT_ACCESS: UiErrorItemConst = UiErrorItemConst::new(
    "Unable to use specified port",
    "Please ensure hulk binary has 'cap_net_bind_service=+ep' permissions",
    "Use 'sudo setcap cap_net_bind_service=+ep /path/to/hulk' to provide sufficient permissions",
);

const SSL_UNEXPECTED_ERROR: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid TLS certificate",
    "Please check the content of your certificate data",
    "Only PEM (x.509) format is accepted as valid public & private certificates",
);

const SSL_UNEXPECTED_DATA: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid TLS certificate",
    "Please check your certificate",
    "",
);

const SSL_NO_PASSWORD: UiErrorItemConst = UiErrorItemConst::new(
    "Missing TLS password",
    "Please set the password to environment variable `HULK_CERT_PASSWD` so that the private key can be decrypted",
    "",
);

const NO_CERTS_AND_HTTPS_ENDPOINTS: UiErrorItemConst = UiErrorItemConst::new(
    "HTTPS specified in endpoints, but no TLS certificate is found on the local machine",
    "Please add TLS certificate or use HTTP endpoints only",
    "Refer to for information about how to load a TLS certificate in your server",
);

const CERTS_AND_HTTP_ENDPOINTS: UiErrorItemConst = UiErrorItemConst::new(
    "HTTP specified in endpoints, but the server in the local machine is configured with a TLS certificate",
    "Please remove the certificate in the configuration directory or switch to HTTPS",
    "",
);

const SSL_WRONG_PASSWORD: UiErrorItemConst = UiErrorItemConst::new(
    "Unable to decrypt the private key using the provided password",
    "Please set the correct password in environment variable `HULK_CERT_PASSWD`",
    "",
);

static UNEXPECTED_ERROR: UiErrorItemConst =
    UiErrorItemConst::new("Unexpected error", "Please contact hulk at", "");

const INVALID_COMPRESSION_INCLUDES_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid compression include value",
    "Please check the passed value",
    "Compress extensions/mime-types are delimited by `,`. For eg, HULK_COMPRESS_MIME_TYPES=\"A,B,C\"",
);

const INVALID_GW_SSE_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid gateway SSE value",
    "Please check the passed value",
    "HULK_GATEWAY_SSE: Gateway SSE accepts only C and S3 as valid values. Delimit by `;` to set more than one value",
);

const INVALID_GW_SSE_ENV_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid gateway SSE configuration",
    "",
    "Refer to for setting up SSE",
);

const INVALID_REPLICATION_WORKERS_VALUE: UiErrorItemConst = UiErrorItemConst::new(
    "Invalid value for replication workers",
    "",
    "HULK_API_REPLICATION_WORKERS: should be > 0",
);
