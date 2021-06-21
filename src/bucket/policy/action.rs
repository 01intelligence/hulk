use std::collections::{HashMap, HashSet};
use std::fmt;

use lazy_static::lazy_static;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::*;

// Policy action.
// Refer https://docs.aws.amazon.com/IAM/latest/UserGuide/list_amazons3.html
// for more information about available actions.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Action<'a>(&'a str);

// ABORT_MULTIPART_UPLOAD_ACTION - AbortMultipartUpload Rest API action.
pub const ABORT_MULTIPART_UPLOAD_ACTION: Action = Action("s3:AbortMultipartUpload");

// CREATE_BUCKET_ACTION - CreateBucket Rest API action.
pub const CREATE_BUCKET_ACTION: Action = Action("s3:CreateBucket");

// DELETE_BUCKET_ACTION - DeleteBucket Rest API action.
pub const DELETE_BUCKET_ACTION: Action = Action("s3:DeleteBucket");

// FORCE_DELETE_BUCKET_ACTION - DeleteBucket Rest API action when x-minio-force-delete flag
// is specified.
pub const FORCE_DELETE_BUCKET_ACTION: Action = Action("s3:ForceDeleteBucket");

// DELETE_BUCKET_POLICY_ACTION - DeleteBucketPolicy Rest API action.
pub const DELETE_BUCKET_POLICY_ACTION: Action = Action("s3:DeleteBucketPolicy");

// DELETE_OBJECT_ACTION - DeleteObject Rest API action.
pub const DELETE_OBJECT_ACTION: Action = Action("s3:DeleteObject");

// GET_BUCKET_LOCATION_ACTION - GetBucketLocation Rest API action.
pub const GET_BUCKET_LOCATION_ACTION: Action = Action("s3:GetBucketLocation");

// GET_BUCKET_NOTIFICATION_ACTION - GetBucketNotification Rest API action.
pub const GET_BUCKET_NOTIFICATION_ACTION: Action = Action("s3:GetBucketNotification");

// GET_BUCKET_POLICY_ACTION - GetBucketPolicy Rest API action.
pub const GET_BUCKET_POLICY_ACTION: Action = Action("s3:GetBucketPolicy");

// GET_OBJECT_ACTION - GetObject Rest API action.
pub const GET_OBJECT_ACTION: Action = Action("s3:GetObject");

// HEAD_BUCKET_ACTION - HeadBucket Rest API action. This action is unused in minio.
pub const HEAD_BUCKET_ACTION: Action = Action("s3:HeadBucket");

// LIST_ALL_MY_BUCKETS_ACTION - ListAllMyBuckets (List buckets) Rest API action.
pub const LIST_ALL_MY_BUCKETS_ACTION: Action = Action("s3:ListAllMyBuckets");

// LIST_BUCKET_ACTION - ListBucket Rest API action.
pub const LIST_BUCKET_ACTION: Action = Action("s3:ListBucket");

// GET_BUCKET_POLICY_STATUS_ACTION - Retrieves the policy status for a bucket.
pub const GET_BUCKET_POLICY_STATUS_ACTION: Action = Action("s3:GetBucketPolicyStatus");

// LIST_BUCKET_MULTIPART_UPLOADS_ACTION - ListMultipartUploads Rest API action.
pub const LIST_BUCKET_MULTIPART_UPLOADS_ACTION: Action = Action("s3:ListBucketMultipartUploads");

// LIST_BUCKET_VERSIONS_ACTION - ListBucket versions Rest API action.
pub const LIST_BUCKET_VERSIONS_ACTION: Action = Action("s3:ListBucketVersions");

// LISTEN_NOTIFICATION_ACTION - ListenNotification Rest API action.
// This is MinIO extension.
pub const LISTEN_NOTIFICATION_ACTION: Action = Action("s3:ListenNotification");

// LISTEN_BUCKET_NOTIFICATION_ACTION - ListenBucketNotification Rest API action.
// This is MinIO extension.
pub const LISTEN_BUCKET_NOTIFICATION_ACTION: Action = Action("s3:ListenBucketNotification");

// LIST_MULTIPART_UPLOAD_PARTS_ACTION - ListParts Rest API action.
pub const LIST_MULTIPART_UPLOAD_PARTS_ACTION: Action = Action("s3:ListMultipartUploadParts");

// PUT_BUCKET_NOTIFICATION_ACTION - PutObjectNotification Rest API action.
pub const PUT_BUCKET_NOTIFICATION_ACTION: Action = Action("s3:PutBucketNotification");

// PUT_BUCKET_POLICY_ACTION - PutBucketPolicy Rest API action.
pub const PUT_BUCKET_POLICY_ACTION: Action = Action("s3:PutBucketPolicy");

// PUT_OBJECT_ACTION - PutObject Rest API action.
pub const PUT_OBJECT_ACTION: Action = Action("s3:PutObject");

// PUT_BUCKET_LIFECYCLE_ACTION - PutBucketLifecycle Rest API action.
pub const PUT_BUCKET_LIFECYCLE_ACTION: Action = Action("s3:PutLifecycleConfiguration");

// GET_BUCKET_LIFECYCLE_ACTION - GetBucketLifecycle Rest API action.
pub const GET_BUCKET_LIFECYCLE_ACTION: Action = Action("s3:GetLifecycleConfiguration");

// BYPASS_GOVERNANCE_RETENTION_ACTION - bypass governance retention for PutObjectRetention, PutObject and DeleteObject Rest API action.
pub const BYPASS_GOVERNANCE_RETENTION_ACTION: Action = Action("s3:BypassGovernanceRetention");
// PUT_OBJECT_RETENTION_ACTION - PutObjectRetention Rest API action.
pub const PUT_OBJECT_RETENTION_ACTION: Action = Action("s3:PutObjectRetention");

// GET_OBJECT_RETENTION_ACTION - GetObjectRetention, GetObject, HeadObject Rest API action.
pub const GET_OBJECT_RETENTION_ACTION: Action = Action("s3:GetObjectRetention");
// GET_OBJECT_LEGAL_HOLD_ACTION - GetObjectLegalHold, GetObject Rest API action.
pub const GET_OBJECT_LEGAL_HOLD_ACTION: Action = Action("s3:GetObjectLegalHold");
// PUT_OBJECT_LEGAL_HOLD_ACTION - PutObjectLegalHold, PutObject Rest API action.
pub const PUT_OBJECT_LEGAL_HOLD_ACTION: Action = Action("s3:PutObjectLegalHold");
// GET_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION - GetObjectLockConfiguration Rest API action
pub const GET_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION: Action =
    Action("s3:GetBucketObjectLockConfiguration");
// PUT_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION - PutObjectLockConfiguration Rest API action
pub const PUT_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION: Action =
    Action("s3:PutBucketObjectLockConfiguration");

// GET_BUCKET_TAGGING_ACTION - GetTagging Rest API action
pub const GET_BUCKET_TAGGING_ACTION: Action = Action("s3:GetBucketTagging");
// PUT_BUCKET_TAGGING_ACTION - PutTagging Rest API action
pub const PUT_BUCKET_TAGGING_ACTION: Action = Action("s3:PutBucketTagging");

// GET_OBJECT_TAGGING_ACTION - Get Object Tags API action
pub const GET_OBJECT_TAGGING_ACTION: Action = Action("s3:GetObjectTagging");
// PUT_OBJECT_TAGGING_ACTION - Put Object Tags API action
pub const PUT_OBJECT_TAGGING_ACTION: Action = Action("s3:PutObjectTagging");
// DELETE_OBJECT_TAGGING_ACTION - Delete Object Tags API action
pub const DELETE_OBJECT_TAGGING_ACTION: Action = Action("s3:DeleteObjectTagging");

// PUT_BUCKET_ENCRYPTION_ACTION - PutBucketEncryption REST API action
pub const PUT_BUCKET_ENCRYPTION_ACTION: Action = Action("s3:PutEncryptionConfiguration");
// GET_BUCKET_ENCRYPTION_ACTION - GetBucketEncryption REST API action
pub const GET_BUCKET_ENCRYPTION_ACTION: Action = Action("s3:GetEncryptionConfiguration");

// PUT_BUCKET_VERSIONING_ACTION - PutBucketVersioning REST API action
pub const PUT_BUCKET_VERSIONING_ACTION: Action = Action("s3:PutBucketVersioning");
// GET_BUCKET_VERSIONING_ACTION - GetBucketVersioning REST API action
pub const GET_BUCKET_VERSIONING_ACTION: Action = Action("s3:GetBucketVersioning");

// DELETE_OBJECT_VERSION_ACTION - DeleteObjectVersion Rest API action.
pub const DELETE_OBJECT_VERSION_ACTION: Action = Action("s3:DeleteObjectVersion");

// DELETE_OBJECT_VERSION_TAGGING_ACTION - DeleteObjectVersionTagging Rest API action.
pub const DELETE_OBJECT_VERSION_TAGGING_ACTION: Action = Action("s3:DeleteObjectVersionTagging");

// GET_OBJECT_VERSION_ACTION - GET_OBJECT_VERSION_ACTION Rest API action.
pub const GET_OBJECT_VERSION_ACTION: Action = Action("s3:GetObjectVersion");

// GET_OBJECT_VERSION_TAGGING_ACTION - GetObjectVersionTagging Rest API action.
pub const GET_OBJECT_VERSION_TAGGING_ACTION: Action = Action("s3:GetObjectVersionTagging");

// PUT_OBJECT_VERSION_TAGGING_ACTION - PutObjectVersionTagging Rest API action.
pub const PUT_OBJECT_VERSION_TAGGING_ACTION: Action = Action("s3:PutObjectVersionTagging");

// GET_REPLICATION_CONFIGURATION_ACTION  - GetReplicationConfiguration REST API action
pub const GET_REPLICATION_CONFIGURATION_ACTION: Action = Action("s3:GetReplicationConfiguration");
// PUT_REPLICATION_CONFIGURATION_ACTION  - PutReplicationConfiguration REST API action
pub const PUT_REPLICATION_CONFIGURATION_ACTION: Action = Action("s3:PutReplicationConfiguration");

// REPLICATE_OBJECT_ACTION  - ReplicateObject REST API action
pub const REPLICATE_OBJECT_ACTION: Action = Action("s3:ReplicateObject");

// REPLICATE_DELETE_ACTION  - ReplicateDelete REST API action
pub const REPLICATE_DELETE_ACTION: Action = Action("s3:ReplicateDelete");

// REPLICATE_TAGS_ACTION  - ReplicateTags REST API action
pub const REPLICATE_TAGS_ACTION: Action = Action("s3:ReplicateTags");

// GET_OBJECT_VERSION_FOR_REPLICATION_ACTION  - GetObjectVersionForReplication REST API action
pub const GET_OBJECT_VERSION_FOR_REPLICATION_ACTION: Action =
    Action("s3:GetObjectVersionForReplication");

// RESTORE_OBJECT_ACTION - RestoreObject REST API action
pub const RESTORE_OBJECT_ACTION: Action = Action("s3:RestoreObject");
// RESET_BUCKET_REPLICATION_STATE_ACTION - MinIO extension API ResetBucketReplicationState to reset replication state
// on a bucket
pub const RESET_BUCKET_REPLICATION_STATE_ACTION: Action = Action("s3:ResetBucketReplicationState");

lazy_static! {
    static ref SUPPORTED_OBJECT_ACTIONS: HashSet<Action<'static>> = maplit::hashset! {
        ABORT_MULTIPART_UPLOAD_ACTION,
        DELETE_OBJECT_ACTION,
        GET_OBJECT_ACTION,
        LIST_MULTIPART_UPLOAD_PARTS_ACTION,
        PUT_OBJECT_ACTION,
        BYPASS_GOVERNANCE_RETENTION_ACTION,
        PUT_OBJECT_RETENTION_ACTION,
        GET_OBJECT_RETENTION_ACTION,
        PUT_OBJECT_LEGAL_HOLD_ACTION,
        GET_OBJECT_LEGAL_HOLD_ACTION,
        GET_OBJECT_TAGGING_ACTION,
        PUT_OBJECT_TAGGING_ACTION,
        DELETE_OBJECT_TAGGING_ACTION,
        GET_OBJECT_VERSION_ACTION,
        GET_OBJECT_VERSION_TAGGING_ACTION,
        DELETE_OBJECT_VERSION_ACTION,
        DELETE_OBJECT_VERSION_TAGGING_ACTION,
        PUT_OBJECT_VERSION_TAGGING_ACTION,
        REPLICATE_OBJECT_ACTION,
        REPLICATE_DELETE_ACTION,
        REPLICATE_TAGS_ACTION,
        GET_OBJECT_VERSION_FOR_REPLICATION_ACTION,
        RESTORE_OBJECT_ACTION,
        RESET_BUCKET_REPLICATION_STATE_ACTION,
    };
    static ref SUPPORTED_ACTIONS: HashSet<Action<'static>> = maplit::hashset! {
        ABORT_MULTIPART_UPLOAD_ACTION,
        CREATE_BUCKET_ACTION,
        DELETE_BUCKET_ACTION,
        FORCE_DELETE_BUCKET_ACTION,
        DELETE_BUCKET_POLICY_ACTION,
        DELETE_OBJECT_ACTION,
        GET_BUCKET_LOCATION_ACTION,
        GET_BUCKET_NOTIFICATION_ACTION,
        GET_BUCKET_POLICY_ACTION,
        GET_OBJECT_ACTION,
        HEAD_BUCKET_ACTION,
        LIST_ALL_MY_BUCKETS_ACTION,
        LIST_BUCKET_ACTION,
        GET_BUCKET_POLICY_STATUS_ACTION,
        LIST_BUCKET_MULTIPART_UPLOADS_ACTION,
        LIST_BUCKET_VERSIONS_ACTION,
        LISTEN_NOTIFICATION_ACTION,
        LISTEN_BUCKET_NOTIFICATION_ACTION,
        LIST_MULTIPART_UPLOAD_PARTS_ACTION,
        PUT_BUCKET_NOTIFICATION_ACTION,
        PUT_BUCKET_POLICY_ACTION,
        PUT_OBJECT_ACTION,
        PUT_BUCKET_LIFECYCLE_ACTION,
        GET_BUCKET_LIFECYCLE_ACTION,
        BYPASS_GOVERNANCE_RETENTION_ACTION,
        PUT_OBJECT_RETENTION_ACTION,
        GET_OBJECT_RETENTION_ACTION,
        GET_OBJECT_LEGAL_HOLD_ACTION,
        PUT_OBJECT_LEGAL_HOLD_ACTION,
        GET_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION,
        PUT_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION,
        GET_BUCKET_TAGGING_ACTION,
        PUT_BUCKET_TAGGING_ACTION,
        GET_OBJECT_TAGGING_ACTION,
        PUT_OBJECT_TAGGING_ACTION,
        DELETE_OBJECT_TAGGING_ACTION,
        PUT_BUCKET_ENCRYPTION_ACTION,
        GET_BUCKET_ENCRYPTION_ACTION,
        PUT_BUCKET_VERSIONING_ACTION,
        GET_BUCKET_VERSIONING_ACTION,
        DELETE_OBJECT_VERSION_ACTION,
        DELETE_OBJECT_VERSION_TAGGING_ACTION,
        GET_OBJECT_VERSION_ACTION,
        GET_OBJECT_VERSION_TAGGING_ACTION,
        PUT_OBJECT_VERSION_TAGGING_ACTION,
        GET_REPLICATION_CONFIGURATION_ACTION,
        PUT_REPLICATION_CONFIGURATION_ACTION,
        REPLICATE_OBJECT_ACTION,
        REPLICATE_DELETE_ACTION,
        REPLICATE_TAGS_ACTION,
        GET_OBJECT_VERSION_FOR_REPLICATION_ACTION,
        RESTORE_OBJECT_ACTION,
        RESET_BUCKET_REPLICATION_STATE_ACTION,
    };
    static ref ACTION_CONDITION_KEY_MAP: HashMap<Action<'static>, condition::KeySet<'static>> = {
        use condition::*;

        use crate::keyset_extend;

        let common_keyset: KeySet<'static> = condition::COMMON_KEYS.iter().cloned().collect();
        maplit::hashmap! {
            ABORT_MULTIPART_UPLOAD_ACTION => common_keyset.clone(),
            CREATE_BUCKET_ACTION => common_keyset.clone(),
            DELETE_OBJECT_ACTION => common_keyset.clone(),
            GET_BUCKET_LOCATION_ACTION => common_keyset.clone(),
            GET_BUCKET_POLICY_STATUS_ACTION => common_keyset.clone(),
            GET_OBJECT_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
            ),
            HEAD_BUCKET_ACTION => common_keyset.clone(),
            LIST_ALL_MY_BUCKETS_ACTION => common_keyset.clone(),
            LIST_BUCKET_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_PREFIX,
                S3_DELIMITER,
                S3_MAX_KEYS,
            ),
            LIST_BUCKET_VERSIONS_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_PREFIX,
                S3_DELIMITER,
                S3_MAX_KEYS,
            ),
            LIST_BUCKET_MULTIPART_UPLOADS_ACTION => common_keyset.clone(),
            LISTEN_NOTIFICATION_ACTION => common_keyset.clone(),

            LISTEN_BUCKET_NOTIFICATION_ACTION => common_keyset.clone(),
            LIST_MULTIPART_UPLOAD_PARTS_ACTION => common_keyset.clone(),
            PUT_OBJECT_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3X_AMZ_COPY_SOURCE,
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
                S3X_AMZ_METADATA_DIRECTIVE,
                S3X_AMZ_STORAGE_CLASS,
                S3_OBJECT_LOCK_RETAIN_UNTIL_DATE,
                S3_OBJECT_LOCK_MODE,
                S3_OBJECT_LOCK_LEGAL_HOLD,
            ),
            // https://docs.aws.amazon.com/AmazonS3/latest/dev/list_amazons3.html
            // LockLegalHold is not supported with PutObjectRetentionAction
            PUT_OBJECT_RETENTION_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_OBJECT_LOCK_REMAINING_RETENTION_DAYS,
                S3_OBJECT_LOCK_RETAIN_UNTIL_DATE,
                S3_OBJECT_LOCK_MODE,
            ),
            GET_OBJECT_RETENTION_ACTION => common_keyset.clone(),
            PUT_OBJECT_LEGAL_HOLD_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_OBJECT_LOCK_LEGAL_HOLD,
            ),
            GET_OBJECT_LEGAL_HOLD_ACTION => common_keyset.clone(),
            // https://docs.aws.amazon.com/AmazonS3/latest/dev/list_amazons3.html
            BYPASS_GOVERNANCE_RETENTION_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_OBJECT_LOCK_REMAINING_RETENTION_DAYS,
                S3_OBJECT_LOCK_RETAIN_UNTIL_DATE,
                S3_OBJECT_LOCK_MODE,
                S3_OBJECT_LOCK_LEGAL_HOLD,
            ),
            GET_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION => common_keyset.clone(),
            PUT_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION => common_keyset.clone(),
            GET_BUCKET_TAGGING_ACTION => common_keyset.clone(),
            PUT_BUCKET_TAGGING_ACTION => common_keyset.clone(),
            PUT_OBJECT_TAGGING_ACTION => common_keyset.clone(),
            GET_OBJECT_TAGGING_ACTION => common_keyset.clone(),
            DELETE_OBJECT_TAGGING_ACTION => common_keyset.clone(),
            PUT_OBJECT_VERSION_TAGGING_ACTION => common_keyset.clone(),
            GET_OBJECT_VERSION_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            GET_OBJECT_VERSION_TAGGING_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            DELETE_OBJECT_VERSION_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            DELETE_OBJECT_VERSION_TAGGING_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            GET_REPLICATION_CONFIGURATION_ACTION => common_keyset.clone(),
            PUT_REPLICATION_CONFIGURATION_ACTION => common_keyset.clone(),
            REPLICATE_OBJECT_ACTION => common_keyset.clone(),
            REPLICATE_DELETE_ACTION => common_keyset.clone(),
            REPLICATE_TAGS_ACTION => common_keyset.clone(),
            GET_OBJECT_VERSION_FOR_REPLICATION_ACTION => common_keyset.clone(),
            RESTORE_OBJECT_ACTION => common_keyset.clone(),
            RESET_BUCKET_REPLICATION_STATE_ACTION => common_keyset.clone(),
        }
    };
}

impl<'a> Action<'a> {
    fn is_object_action(&self) -> bool {
        SUPPORTED_OBJECT_ACTIONS.contains(self)
    }
}

impl<'a> Valid for Action<'a> {
    fn is_valid(&self) -> bool {
        SUPPORTED_ACTIONS.contains(self)
    }
}

impl<'a> Serialize for Action<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if !self.is_valid() {
            return Err(S::Error::custom(format!("invalid action '{}'", self.0)));
        }
        serializer.serialize_str(self.0)
    }
}

impl<'de, 'a> Deserialize<'de> for Action<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ActionVisitor;
        impl<'de> Visitor<'de> for ActionVisitor {
            type Value = Action<'static>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an action")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                SUPPORTED_ACTIONS
                    .iter()
                    .find(|&a| a.0 == v)
                    .cloned()
                    .ok_or(E::custom(format!("invalid action '{}'", v)))
            }
        }

        deserializer.deserialize_str(ActionVisitor)
    }
}

pub struct ActionSet<'a>(HashSet<Action<'a>>);
