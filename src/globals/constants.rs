use std::time::Duration;

// Configuration related constants.
pub const GLOBAL_DEFAULT_PORT: &str = "9000";

pub const GLOBAL_DEFAULT_REGION: &str = "";
// This is a sha256 output of ``arn:aws:iam::hulk:user/admin``,
// this is kept in present form to be compatible with S3 owner ID
// requirements -
//
// ```
//    The canonical user ID is the Amazon S3–only concept.
//    It is 64-character obfuscated version of the account ID.
// ```
// http://docs.aws.amazon.com/AmazonS3/latest/dev/example-walkthroughs-managing-access-example4.html
pub const GLOBAL_DEFAULT_OWNER_ID: &str =
    "786914333986fba80e900e88556d33e97b688e479f7cc38a59982ee7ccbc42b9";
pub const GLOBAL_DEFAULT_STORAGE_CLASS: &str = "STANDARD";
pub const GLOBAL_WINDOWS_OSNAME: &str = "windows";
pub const GLOBAL_MAC_OSNAME: &str = "darwin";
pub const GLOBAL_MODE_FS: &str = "mode-server-fs";
pub const GLOBAL_MODE_ERASURE: &str = "mode-server-xl";
pub const GLOBAL_MODE_DIST_ERASURE: &str = "mode-server-distributed-xl";
pub const GLOBAL_MODE_GATEWAY_PREFIX: &str = "mode-gateway-";
pub const GLOBAL_DIR_SUFFIX: &str = "__XLDIR__";
pub const GLOBAL_DIR_SUFFIX_WITH_SLASH: &str = "__XLDIR__/";

pub const SLASH_SEPARATOR: &str = "/";

const MIB: usize = 1 << (2 * 10);
const MINUTE: u64 = 60;
const HOUR: u64 = MINUTE * 60;

// Limit fields size (except file) to 1Mib since Policy document
// can reach that size according to https://aws.amazon.com/articles/1434
const MAX_FORM_FIELD_SIZE: usize = MIB;

// Limit memory allocation to store multipart data
const MAX_FORM_MEMORY: usize = MIB * 5;

// The maximum allowed time difference between the incoming request
// date and server date during signature verification.
const GLOBAL_MAX_SKEW_TIME: Duration = Duration::from_secs(MINUTE * 15); // 15 minutes skew allowed.

// EXPIRY - Expiry duration after which the uploads in multipart, tmp directory are deemed stale.
const GLOBAL_STALE_UPLOADS_EXPIRY: Duration = Duration::from_secs(HOUR * 24); // 24 hrs.

// Cleanup interval when the stale uploads cleanup is initiated.
const GLOBAL_STALE_UPLOADS_CLEANUP_INTERVAL: Duration = Duration::from_secs(HOUR * 12); // 12 hrs.

// Executes the Lifecycle events.
const GLOBAL_SERVICE_EXECUTION_INTERVAL: Duration = Duration::from_secs(HOUR * 24); // 24 hrs.

// Refresh interval to update in-memory iam config cache.
const GLOBAL_REFRESH_IAM_INTERVAL: Duration = Duration::from_secs(MINUTE * 5);

// Limit of location constraint XML for unauthenticated PUT bucket operations.
const MAX_LOCATION_CONSTRAINT_SIZE: usize = MIB * 3;

// Maximum size of default bucket encryption configuration allowed
const MAX_BUCKET_SSE_CONFIG_SIZE: usize = MIB;

// The fraction of a disk we allow to be filled.
const DISK_FILL_FRACTION: f64 = 0.95;

// The size to assume when an unknown size upload is requested.
const DISK_ASSUME_UNKNOWN_SIZE: usize = 1 << 30;

// The minimum number of inodes we want free on a disk to perform writes.
const DISK_MIN_INODES: usize = 1000;