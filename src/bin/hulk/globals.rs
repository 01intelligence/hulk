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
