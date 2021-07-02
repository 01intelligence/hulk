use std::any::Any;

use colored::*;
use lazy_static::lazy_static;
use thiserror::Error;

use crate::config::*;

#[derive(Clone, Debug, Default)]
pub struct UiError {
    msg: String,
    action: String,
    hint: String,
    detail: Option<String>,
}

impl UiError {
    fn new(msg: &str, action: &str, hint: &str) -> UiError {
        UiError {
            msg: msg.to_owned(),
            action: action.to_owned(),
            hint: hint.to_owned(),
            detail: None,
        }
    }

    fn from_error<E: std::error::Error + 'static>(err: Option<E>) -> UiError {
        match err {
            None => UiError::default(),
            Some(err) => {
                let e = &err as &dyn Any;
                if let Some(e) = e.downcast_ref::<UiError>() {
                    return e.clone();
                }
                // TODO: downcast to other error types
                return UiError {
                    msg: err.to_string(),
                    ..Default::default()
                };
            }
        }
    }

    pub fn msg(&self, msg: String) -> UiError {
        let mut e = self.clone();
        e.msg = msg;
        e
    }

    pub fn hint(&self, hint: String) -> UiError {
        let mut e = self.clone();
        e.hint = hint;
        e
    }

    pub fn error(&self, err: anyhow::Error) -> UiError {
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

lazy_static! {
    pub static ref UiErrorInvalidBrowserValue: UiError = UiError::new(
        "Invalid browser value",
        "Please check the passed value",
        "Browser can only accept `on` and `off` values. To disable web browser access, set this value to `off`",
    );

    pub static ref UiErrorInvalidFSOSyncValue: UiError = UiError::new(
        "Invalid O_SYNC value",
        "Please check the passed value",
        "Can only accept `on` and `off` values. To enable O_SYNC for fs backend, set this value to `on`",
    );

    pub static ref UiErrorOverlappingDomainValue: UiError = UiError::new(
        "Overlapping domain values",
        "Please check the passed value",
        "MINIO_DOMAIN only accepts non-overlapping domain values",
    );

    pub static ref UiErrorInvalidDomainValue: UiError = UiError::new(
        "Invalid domain value",
        "Please check the passed value",
        "Domain can only accept DNS compatible values",
    );

    pub static ref UiErrorInvalidErasureSetSize: UiError = UiError::new(
        "Invalid erasure set size",
        "Please check the passed value",
        "Erasure set can only accept any of [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] values",
    );

    pub static ref UiErrorInvalidWormValue: UiError = UiError::new(
        "Invalid WORM value",
        "Please check the passed value",
        "WORM can only accept `on` and `off` values. To enable WORM, set this value to `on`",
    );

    pub static ref UiErrorInvalidCacheDrivesValue: UiError = UiError::new(
        "Invalid cache drive value",
        "Please check the value in this ENV variable",
        "MINIO_CACHE_DRIVES: Mounted drives or directories are delimited by `,`",
    );

    pub static ref UiErrorInvalidCacheExcludesValue: UiError = UiError::new(
        "Invalid cache excludes value",
        "Please check the passed value",
        "MINIO_CACHE_EXCLUDE: Cache exclusion patterns are delimited by `,`",
    );

    pub static ref UiErrorInvalidCacheExpiryValue: UiError = UiError::new(
        "Invalid cache expiry value",
        "Please check the passed value",
        "MINIO_CACHE_EXPIRY: Valid cache expiry duration must be in days",
    );

    pub static ref UiErrorInvalidCacheQuota: UiError = UiError::new(
        "Invalid cache quota value",
        "Please check the passed value",
        "MINIO_CACHE_QUOTA: Valid cache quota value must be between 0-100",
    );

    pub static ref UiErrorInvalidCacheAfter: UiError = UiError::new(
        "Invalid cache after value",
        "Please check the passed value",
        "MINIO_CACHE_AFTER: Valid cache after value must be 0 or greater",
    );

    pub static ref UiErrorInvalidCacheWatermarkLow: UiError = UiError::new(
        "Invalid cache low watermark value",
        "Please check the passed value",
        "MINIO_CACHE_WATERMARK_LOW: Valid cache low watermark value must be between 0-100",
    );

    pub static ref UiErrorInvalidCacheWatermarkHigh: UiError = UiError::new(
        "Invalid cache high watermark value",
        "Please check the passed value",
        "MINIO_CACHE_WATERMARK_HIGH: Valid cache high watermark value must be between 0-100",
    );

    pub static ref UiErrorInvalidCacheEncryptionKey: UiError = UiError::new(
        "Invalid cache encryption master key value",
        "Please check the passed value",
        "MINIO_CACHE_ENCRYPTION_MASTER_KEY: For more information, please refer to https://docs.min.io/docs/minio-disk-cache-guide",
    );

    pub static ref UiErrorInvalidCacheRange: UiError = UiError::new(
        "Invalid cache range value",
        "Please check the passed value",
        "MINIO_CACHE_RANGE: Valid expected value is `on` or `off`",
    );

    pub static ref UiErrorInvalidCacheCommitValue: UiError = UiError::new(
        "Invalid cache commit value",
        "Please check the passed value",
        "MINIO_CACHE_COMMIT: Valid expected value is `writeback` or `writethrough`",
    );

    pub static ref UiErrorInvalidCacheSetting: UiError = UiError::new(
        "Incompatible cache setting",
        "Please check the passed value",
        "MINIO_CACHE_AFTER cannot be used with MINIO_CACHE_COMMIT setting",
    );

    pub static ref UiErrorInvalidCredentialsBackendEncrypted: UiError = UiError::new(
        "Invalid credentials",
        "Please set correct credentials in the environment for decryption",
        "Detected encrypted config backend, correct access and secret keys should be specified via environment variables MINIO_ROOT_USER and MINIO_ROOT_PASSWORD to be able to decrypt the MinIO config, user IAM and policies",
    );

    pub static ref UiErrorInvalidCredentials: UiError = UiError::new(
        "Invalid credentials",
        "Please provide correct credentials",
        "Access key length should be at least 3, and secret key length at least 8 characters",
    );

    pub static ref UiErrorMissingEnvCredentialRootUser: UiError = UiError::new(
        &format!("Missing credential environment variable, \"{}\"", ENV_ROOT_USER),
        &format!("Environment variable \"{}\" is missing", ENV_ROOT_USER),
        "Root user name (access key) and root password (secret key) are expected to be specified via environment variables MINIO_ROOT_USER and MINIO_ROOT_PASSWORD respectively",
    );

    pub static ref UiErrorMissingEnvCredentialRootPassword: UiError = UiError::new(
        &format!("Missing credential environment variable, \"{}\"", ENV_ROOT_PASSWORD),
        &format!("Environment variable \"{}\" is missing", ENV_ROOT_PASSWORD),
        "Root user name (access key) and root password (secret key) are expected to be specified via environment variables MINIO_ROOT_USER and MINIO_ROOT_PASSWORD respectively",
    );

    pub static ref UiErrorMissingEnvCredentialAccessKey: UiError = UiError::new(
        &format!("Missing credential environment variable, \"{}\"", ENV_ACCESS_KEY),
        &format!("Environment variables \"{}\" and \"{}\" are deprecated", ENV_ACCESS_KEY, ENV_SECRET_KEY),
        "Root user name (access key) and root password (secret key) are expected to be specified via environment variables MINIO_ROOT_USER and MINIO_ROOT_PASSWORD respectively",
    );

    pub static ref UiErrorMissingEnvCredentialSecretKey: UiError = UiError::new(
        &format!("Missing credential environment variable, \"{}\"", ENV_SECRET_KEY),
        &format!("Environment variables \"{}\" and \"{}\" are deprecated", ENV_SECRET_KEY, ENV_ACCESS_KEY),
        "Root user name (access key) and root password (secret key) are expected to be specified via environment variables MINIO_ROOT_USER and MINIO_ROOT_PASSWORD respectively",
    );

    pub static ref UiErrorInvalidErasureEndpoints: UiError = UiError::new(
        "Invalid endpoint(s) in erasure mode",
        "Please provide correct combination of local/remote paths",
        "For more information, please refer to https://docs.min.io/docs/minio-erasure-code-quickstart-guide",
    );

    pub static ref UiErrorInvalidNumberOfErasureEndpoints: UiError = UiError::new(
        "Invalid total number of endpoints for erasure mode",
        "Please provide an even number of endpoints greater or equal to 4",
        "For more information, please refer to https://docs.min.io/docs/minio-erasure-code-quickstart-guide",
    );

    pub static ref UiErrorStorageClassValue: UiError = UiError::new(
        "Invalid storage class value",
        "Please check the value",
        r#"MINIO_STORAGE_CLASS_STANDARD: Format "EC:<Default_Parity_Standard_Class>" (e.g. "EC:3"). This sets the number of parity disks for MinIO server in Standard mode. Objects are stored in Standard mode, if storage class is not defined in Put request
MINIO_STORAGE_CLASS_RRS: Format "EC:<Default_Parity_Reduced_Redundancy_Class>" (e.g. "EC:3"). This sets the number of parity disks for MinIO server in Reduced Redundancy mode. Objects are stored in Reduced Redundancy mode, if Put request specifies RRS storage class
Refer to the link https://github.com/minio/minio/tree/master/docs/erasure/storage-class for more information"#,
    );

    pub static ref UiErrorUnexpectedBackendVersion: UiError = UiError::new(
        "Backend version seems to be too recent",
        "Please update to the latest MinIO version",
        "",
    );

    pub static ref UiErrorInvalidAddressFlag: UiError = UiError::new(
        "--address input is invalid",
        "Please check --address parameter",
        r"--address binds to a specific ADDRESS:PORT, ADDRESS can be an IPv4/IPv6 address or hostname (default port is ':9000')
	Examples: --address ':443'
		  --address '172.16.34.31:9000'
		  --address '[fe80::da00:a6c8:e3ae:ddd7]:9000'",
    );

    pub static ref UiErrorInvalidFSEndpoint: UiError = UiError::new(
        "Invalid endpoint for standalone FS mode",
        "Please check the FS endpoint",
        r"FS mode requires only one writable disk path
Example 1:
   $ minio server /data/minio/",
    );

    pub static ref UiErrorUnsupportedBackend: UiError = UiError::new(
        "Unable to write to the backend",
        "Please ensure your disk supports O_DIRECT",
        "",
    );

    pub static ref UiErrorUnableToWriteInBackend: UiError = UiError::new(
        "Unable to write to the backend",
        "Please ensure MinIO binary has write permissions for the backend",
        "Verify if MinIO binary is running as the same user who has write permissions for the backend",
    );

    pub static ref UiErrorPortAlreadyInUse: UiError = UiError::new(
        "Port is already in use",
        "Please ensure no other program uses the same address/port",
        "",
    );

    pub static ref UiErrorPortAccess: UiError = UiError::new(
        "Unable to use specified port",
        "Please ensure MinIO binary has 'cap_net_bind_service=+ep' permissions",
        "Use 'sudo setcap cap_net_bind_service=+ep /path/to/minio' to provide sufficient permissions",
    );

    pub static ref UiErrorSSLUnexpectedError: UiError = UiError::new(
        "Invalid TLS certificate",
        "Please check the content of your certificate data",
        "Only PEM (x.509) format is accepted as valid public & private certificates",
    );

    pub static ref UiErrorSSLUnexpectedData: UiError = UiError::new(
        "Invalid TLS certificate",
        "Please check your certificate",
        "",
    );

    pub static ref UiErrorSSLNoPassword: UiError = UiError::new(
        "Missing TLS password",
        "Please set the password to environment variable `MINIO_CERT_PASSWD` so that the private key can be decrypted",
        "",
    );

    pub static ref UiErrorNoCertsAndHTTPSEndpoints: UiError = UiError::new(
        "HTTPS specified in endpoints, but no TLS certificate is found on the local machine",
        "Please add TLS certificate or use HTTP endpoints only",
        "Refer to https://docs.min.io/docs/how-to-secure-access-to-minio-server-with-tls for information about how to load a TLS certificate in your server",
    );

    pub static ref UiErrorCertsAndHTTPEndpoints: UiError = UiError::new(
        "HTTP specified in endpoints, but the server in the local machine is configured with a TLS certificate",
        "Please remove the certificate in the configuration directory or switch to HTTPS",
        "",
    );

    pub static ref UiErrorSSLWrongPassword: UiError = UiError::new(
        "Unable to decrypt the private key using the provided password",
        "Please set the correct password in environment variable `MINIO_CERT_PASSWD`",
        "",
    );

    pub static ref UiErrorUnexpectedError: UiError = UiError::new(
        "Unexpected error",
        "Please contact MinIO at https://slack.min.io",
        "",
    );

    pub static ref UiErrorInvalidCompressionIncludesValue: UiError = UiError::new(
        "Invalid compression include value",
        "Please check the passed value",
        "Compress extensions/mime-types are delimited by `,`. For eg, MINIO_COMPRESS_MIME_TYPES=\"A,B,C\"",
    );

    pub static ref UiErrorInvalidGWSSEValue: UiError = UiError::new(
        "Invalid gateway SSE value",
        "Please check the passed value",
        "MINIO_GATEWAY_SSE: Gateway SSE accepts only C and S3 as valid values. Delimit by `;` to set more than one value",
    );

    pub static ref UiErrorInvalidGWSSEEnvValue: UiError = UiError::new(
        "Invalid gateway SSE configuration",
        "",
        "Refer to https://docs.min.io/docs/minio-kms-quickstart-guide.html for setting up SSE",
    );

    pub static ref UiErrorInvalidReplicationWorkersValue: UiError = UiError::new(
        "Invalid value for replication workers",
        "",
        "MINIO_API_REPLICATION_WORKERS: should be > 0",
    );
}
