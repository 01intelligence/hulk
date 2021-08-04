use const_format::concatcp;

use crate::globals;

pub const HEALTH_CHECK_PATH: &str = "/health";
pub const HEALTH_CHECK_LIVENESS_PATH: &str = "/live";
pub const HEALTH_CHECK_READINESS_PATH: &str = "/ready";
pub const HEALTH_CHECK_CLUSTER_PATH: &str = "/cluster";
pub const HEALTH_CHECK_CLUSTER_READ_PATH: &str = "/cluster/read";
pub const HEALTH_CHECK_PATH_PREFIX: &str =
    concatcp!(globals::SYSTEM_RESERVED_BUCKET_PATH, HEALTH_CHECK_PATH);
