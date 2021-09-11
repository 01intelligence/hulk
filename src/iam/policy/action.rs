use std::borrow::Cow;
use std::collections::{hash_map, hash_set, HashMap, HashSet};
use std::fmt;
use std::ops::Deref;

use anyhow::bail;
use lazy_static::lazy_static;
use serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};

use super::*;
use crate::bucket::policy::{condition, Valid};

// Policy action.
// Refer https://docs.aws.amazon.com/IAM/latest/UserGuide/list_amazons3.html
// for more information about available actions.
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Debug)]
pub struct Action<'a>(pub(super) Cow<'a, str>);

// ABORT_MULTIPART_UPLOAD_ACTION - AbortMultipartUpload Rest API action.
pub const ABORT_MULTIPART_UPLOAD_ACTION: &str = "s3:AbortMultipartUpload";

// CREATE_BUCKET_ACTION - CreateBucket Rest API action.
pub const CREATE_BUCKET_ACTION: &str = "s3:CreateBucket";

// DELETE_BUCKET_ACTION - DeleteBucket Rest API action.
pub const DELETE_BUCKET_ACTION: &str = "s3:DeleteBucket";

// FORCE_DELETE_BUCKET_ACTION - DeleteBucket Rest API action when x-hulk-force-delete flag
// is specified.
pub const FORCE_DELETE_BUCKET_ACTION: &str = "s3:ForceDeleteBucket";

// DELETE_BUCKET_POLICY_ACTION - DeleteBucketPolicy Rest API action.
pub const DELETE_BUCKET_POLICY_ACTION: &str = "s3:DeleteBucketPolicy";

// DELETE_OBJECT_ACTION - DeleteObject Rest API action.
pub const DELETE_OBJECT_ACTION: &str = "s3:DeleteObject";

// GET_BUCKET_LOCATION_ACTION - GetBucketLocation Rest API action.
pub const GET_BUCKET_LOCATION_ACTION: &str = "s3:GetBucketLocation";

// GET_BUCKET_NOTIFICATION_ACTION - GetBucketNotification Rest API action.
pub const GET_BUCKET_NOTIFICATION_ACTION: &str = "s3:GetBucketNotification";

// GET_BUCKET_POLICY_ACTION - GetBucketPolicy Rest API action.
pub const GET_BUCKET_POLICY_ACTION: &str = "s3:GetBucketPolicy";

// GET_OBJECT_ACTION - GetObject Rest API action.
pub const GET_OBJECT_ACTION: &str = "s3:GetObject";

// HEAD_BUCKET_ACTION - HeadBucket Rest API action. This action is unused in hulk.
pub const HEAD_BUCKET_ACTION: &str = "s3:HeadBucket";

// LIST_ALL_MY_BUCKETS_ACTION - ListAllMyBuckets (List buckets) Rest API action.
pub const LIST_ALL_MY_BUCKETS_ACTION: &str = "s3:ListAllMyBuckets";

// LIST_BUCKET_ACTION - ListBucket Rest API action.
pub const LIST_BUCKET_ACTION: &str = "s3:ListBucket";

// GET_BUCKET_POLICY_STATUS_ACTION - Retrieves the policy status for a bucket.
pub const GET_BUCKET_POLICY_STATUS_ACTION: &str = "s3:GetBucketPolicyStatus";

// LIST_BUCKET_MULTIPART_UPLOADS_ACTION - ListMultipartUploads Rest API action.
pub const LIST_BUCKET_MULTIPART_UPLOADS_ACTION: &str = "s3:ListBucketMultipartUploads";

// LIST_BUCKET_VERSIONS_ACTION - ListBucket versions Rest API action.
pub const LIST_BUCKET_VERSIONS_ACTION: &str = "s3:ListBucketVersions";

// LISTEN_NOTIFICATION_ACTION - ListenNotification Rest API action.
// This is hulk extension.
pub const LISTEN_NOTIFICATION_ACTION: &str = "s3:ListenNotification";

// LISTEN_BUCKET_NOTIFICATION_ACTION - ListenBucketNotification Rest API action.
// This is hulk extension.
pub const LISTEN_BUCKET_NOTIFICATION_ACTION: &str = "s3:ListenBucketNotification";

// LIST_MULTIPART_UPLOAD_PARTS_ACTION - ListParts Rest API action.
pub const LIST_MULTIPART_UPLOAD_PARTS_ACTION: &str = "s3:ListMultipartUploadParts";

// PUT_BUCKET_LIFECYCLE_ACTION - PutBucketLifecycle Rest API action.
pub const PUT_BUCKET_LIFECYCLE_ACTION: &str = "s3:PutLifecycleConfiguration";

// GET_BUCKET_LIFECYCLE_ACTION - GetBucketLifecycle Rest API action.
pub const GET_BUCKET_LIFECYCLE_ACTION: &str = "s3:GetLifecycleConfiguration";

// PUT_BUCKET_NOTIFICATION_ACTION - PutObjectNotification Rest API action.
pub const PUT_BUCKET_NOTIFICATION_ACTION: &str = "s3:PutBucketNotification";

// PUT_BUCKET_POLICY_ACTION - PutBucketPolicy Rest API action.
pub const PUT_BUCKET_POLICY_ACTION: &str = "s3:PutBucketPolicy";

// PUT_OBJECT_ACTION - PutObject Rest API action.
pub const PUT_OBJECT_ACTION: &str = "s3:PutObject";

// DELETE_OBJECT_VERSION_ACTION - DeleteObjectVersion Rest API action.
pub const DELETE_OBJECT_VERSION_ACTION: &str = "s3:DeleteObjectVersion";

// DELETE_OBJECT_VERSION_TAGGING_ACTION - DeleteObjectVersionTagging Rest API action.
pub const DELETE_OBJECT_VERSION_TAGGING_ACTION: &str = "s3:DeleteObjectVersionTagging";

// GET_OBJECT_VERSION_ACTION - GET_OBJECT_VERSION_ACTION Rest API action.
pub const GET_OBJECT_VERSION_ACTION: &str = "s3:GetObjectVersion";

// GET_OBJECT_VERSION_TAGGING_ACTION - GetObjectVersionTagging Rest API action.
pub const GET_OBJECT_VERSION_TAGGING_ACTION: &str = "s3:GetObjectVersionTagging";

// PUT_OBJECT_VERSION_TAGGING_ACTION - PutObjectVersionTagging Rest API action.
pub const PUT_OBJECT_VERSION_TAGGING_ACTION: &str = "s3:PutObjectVersionTagging";

// BYPASS_GOVERNANCE_RETENTION_ACTION - bypass governance retention for PutObjectRetention, PutObject and DeleteObject Rest API action.
pub const BYPASS_GOVERNANCE_RETENTION_ACTION: &str = "s3:BypassGovernanceRetention";

// PUT_OBJECT_RETENTION_ACTION - PutObjectRetention Rest API action.
pub const PUT_OBJECT_RETENTION_ACTION: &str = "s3:PutObjectRetention";

// GET_OBJECT_RETENTION_ACTION - GetObjectRetention, GetObject, HeadObject Rest API action.
pub const GET_OBJECT_RETENTION_ACTION: &str = "s3:GetObjectRetention";

// GET_OBJECT_LEGAL_HOLD_ACTION - GetObjectLegalHold, GetObject Rest API action.
pub const GET_OBJECT_LEGAL_HOLD_ACTION: &str = "s3:GetObjectLegalHold";
// PUT_OBJECT_LEGAL_HOLD_ACTION - PutObjectLegalHold, PutObject Rest API action.
pub const PUT_OBJECT_LEGAL_HOLD_ACTION: &str = "s3:PutObjectLegalHold";

// GET_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION - GetObjectLockConfiguration Rest API action
pub const GET_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION: &str = "s3:GetBucketObjectLockConfiguration";
// PUT_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION - PutObjectLockConfiguration Rest API action
pub const PUT_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION: &str = "s3:PutBucketObjectLockConfiguration";

// GET_BUCKET_TAGGING_ACTION - GetTagging Rest API action
pub const GET_BUCKET_TAGGING_ACTION: &str = "s3:GetBucketTagging";
// PUT_BUCKET_TAGGING_ACTION - PutTagging Rest API action
pub const PUT_BUCKET_TAGGING_ACTION: &str = "s3:PutBucketTagging";

// GET_OBJECT_TAGGING_ACTION - Get Object Tags API action
pub const GET_OBJECT_TAGGING_ACTION: &str = "s3:GetObjectTagging";
// PUT_OBJECT_TAGGING_ACTION - Put Object Tags API action
pub const PUT_OBJECT_TAGGING_ACTION: &str = "s3:PutObjectTagging";
// DELETE_OBJECT_TAGGING_ACTION - Delete Object Tags API action
pub const DELETE_OBJECT_TAGGING_ACTION: &str = "s3:DeleteObjectTagging";

// PUT_BUCKET_ENCRYPTION_ACTION - PutBucketEncryption REST API action
pub const PUT_BUCKET_ENCRYPTION_ACTION: &str = "s3:PutEncryptionConfiguration";

// GET_BUCKET_ENCRYPTION_ACTION - GetBucketEncryption REST API action
pub const GET_BUCKET_ENCRYPTION_ACTION: &str = "s3:GetEncryptionConfiguration";

// PUT_BUCKET_VERSIONING_ACTION - PutBucketVersioning REST API action
pub const PUT_BUCKET_VERSIONING_ACTION: &str = "s3:PutBucketVersioning";

// GET_BUCKET_VERSIONING_ACTION - GetBucketVersioning REST API action
pub const GET_BUCKET_VERSIONING_ACTION: &str = "s3:GetBucketVersioning";
// GET_REPLICATION_CONFIGURATION_ACTION  - GetReplicationConfiguration REST API action
pub const GET_REPLICATION_CONFIGURATION_ACTION: &str = "s3:GetReplicationConfiguration";
// PUT_REPLICATION_CONFIGURATION_ACTION  - PutReplicationConfiguration REST API action
pub const PUT_REPLICATION_CONFIGURATION_ACTION: &str = "s3:PutReplicationConfiguration";

// REPLICATE_OBJECT_ACTION  - ReplicateObject REST API action
pub const REPLICATE_OBJECT_ACTION: &str = "s3:ReplicateObject";

// REPLICATE_DELETE_ACTION  - ReplicateDelete REST API action
pub const REPLICATE_DELETE_ACTION: &str = "s3:ReplicateDelete";

// REPLICATE_TAGS_ACTION  - ReplicateTags REST API action
pub const REPLICATE_TAGS_ACTION: &str = "s3:ReplicateTags";

// GET_OBJECT_VERSION_FOR_REPLICATION_ACTION  - GetObjectVersionForReplication REST API action
pub const GET_OBJECT_VERSION_FOR_REPLICATION_ACTION: &str = "s3:GetObjectVersionForReplication";

// ALL_ACTIONS - all API actions
pub const ALL_ACTIONS: &str = "s3:*";

lazy_static! {
    static ref SUPPORTED_ACTIONS: HashSet<Action<'static>> = (maplit::hashset! {
        ABORT_MULTIPART_UPLOAD_ACTION,
        CREATE_BUCKET_ACTION,
        DELETE_OBJECT_ACTION,
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
        LIST_BUCKET_VERSIONS_ACTION,
        LIST_BUCKET_MULTIPART_UPLOADS_ACTION,
        LISTEN_NOTIFICATION_ACTION,
        LISTEN_BUCKET_NOTIFICATION_ACTION,
        LIST_MULTIPART_UPLOAD_PARTS_ACTION,
        PUT_BUCKET_LIFECYCLE_ACTION,
        GET_BUCKET_LIFECYCLE_ACTION,
        PUT_BUCKET_NOTIFICATION_ACTION,
        PUT_BUCKET_POLICY_ACTION,
        PUT_OBJECT_ACTION,
        BYPASS_GOVERNANCE_RETENTION_ACTION,
        PUT_OBJECT_RETENTION_ACTION,
        GET_OBJECT_RETENTION_ACTION,
        PUT_OBJECT_LEGAL_HOLD_ACTION,
        GET_OBJECT_LEGAL_HOLD_ACTION,
        GET_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION,
        PUT_BUCKET_OBJECT_LOCK_CONFIGURATION_ACTION,
        GET_BUCKET_TAGGING_ACTION,
        PUT_BUCKET_TAGGING_ACTION,
        GET_OBJECT_VERSION_ACTION,
        GET_OBJECT_VERSION_TAGGING_ACTION,
        DELETE_OBJECT_VERSION_ACTION,
        DELETE_OBJECT_VERSION_TAGGING_ACTION,
        PUT_OBJECT_VERSION_TAGGING_ACTION,
        GET_OBJECT_TAGGING_ACTION,
        PUT_OBJECT_TAGGING_ACTION,
        DELETE_OBJECT_TAGGING_ACTION,
        PUT_BUCKET_ENCRYPTION_ACTION,
        GET_BUCKET_ENCRYPTION_ACTION,
        PUT_BUCKET_VERSIONING_ACTION,
        GET_BUCKET_VERSIONING_ACTION,
        GET_REPLICATION_CONFIGURATION_ACTION,
        PUT_REPLICATION_CONFIGURATION_ACTION,
        REPLICATE_OBJECT_ACTION,
        REPLICATE_DELETE_ACTION,
        REPLICATE_TAGS_ACTION,
        GET_OBJECT_VERSION_FOR_REPLICATION_ACTION,
        ALL_ACTIONS,
    }).into_iter().map(|v| v.into()).collect();

    static ref SUPPORTED_OBJECT_ACTIONS: HashSet<Action<'static>> = (maplit::hashset! {
        ALL_ACTIONS,
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
    }).into_iter().map(|v| v.into()).collect();

    // Holds mapping of supported condition key for an action.
    pub(super) static ref IAM_ACTION_CONDITION_KEY_MAP: HashMap<Action<'static>, condition::KeySet<'static>> = {
        use crate::bucket::policy::condition::*;

        use crate::keyset_extend;

        let common_keyset: KeySet<'static> = condition::COMMON_KEYS.iter().cloned().collect();
        let all_actions: KeySet<'static> = condition::ALL_SUPPORTED_KEYS.iter().cloned().collect();
        (maplit::hashmap! {
            ALL_ACTIONS => all_actions,
            GET_OBJECT_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
                S3_VERSION_ID,
            ),
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
            DELETE_OBJECT_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            PUT_OBJECT_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3X_AMZ_COPY_SOURCE,
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
                S3X_AMZ_METADATA_DIRECTIVE,
                S3X_AMZ_STORAGE_CLASS,
                S3_VERSION_ID,
                S3_OBJECT_LOCK_RETAIN_UNTIL_DATE,
                S3_OBJECT_LOCK_MODE,
                S3_OBJECT_LOCK_LEGAL_HOLD,
            ),
            // https://docs.aws.amazon.com/AmazonS3/latest/dev/list_amazons3.html
            // LockLegalHold is not supported with PutObjectRetentionAction
            PUT_OBJECT_RETENTION_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
                S3_OBJECT_LOCK_REMAINING_RETENTION_DAYS,
                S3_OBJECT_LOCK_RETAIN_UNTIL_DATE,
                S3_OBJECT_LOCK_MODE,
                S3_VERSION_ID,
            ),
            GET_OBJECT_RETENTION_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
                S3_VERSION_ID,
            ),
            PUT_OBJECT_LEGAL_HOLD_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
                S3_OBJECT_LOCK_LEGAL_HOLD,
                S3_VERSION_ID,
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
            PUT_OBJECT_TAGGING_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            GET_OBJECT_TAGGING_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            DELETE_OBJECT_TAGGING_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            PUT_OBJECT_VERSION_TAGGING_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
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
            REPLICATE_OBJECT_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            REPLICATE_DELETE_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            REPLICATE_TAGS_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
            GET_OBJECT_VERSION_FOR_REPLICATION_ACTION => keyset_extend!(
                common_keyset.clone(),
                S3_VERSION_ID,
            ),
        }).into_iter().map(|(k, v)| (k.into(), v)).collect()
    };
}

pub(super) fn action_condition_keyset(action: &Action) -> condition::KeySet<'static> {
    let mut ks_merged: condition::KeySet<'static> =
        condition::COMMON_KEYS.iter().cloned().collect();
    for (a, ks) in IAM_ACTION_CONDITION_KEY_MAP.iter() {
        if action.is_match(a) {
            ks_merged.extend(ks.iter().cloned());
        }
    }
    ks_merged
}

impl<'a> Action<'a> {
    pub(super) fn is_object_action(&self) -> bool {
        SUPPORTED_OBJECT_ACTIONS.iter().any(|a| self.is_match(a))
    }
    pub fn is_match(&self, a: &Action) -> bool {
        crate::wildcard::match_wildcard(&self, &a)
    }
}

impl<'a> Deref for Action<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<'a> From<&'a str> for Action<'a> {
    fn from(a: &'a str) -> Self {
        Action(a.into())
    }
}

impl From<String> for Action<'_> {
    fn from(a: String) -> Self {
        Action(a.into())
    }
}

impl<'a> Valid for Action<'a> {
    fn is_valid(&self) -> bool {
        SUPPORTED_ACTIONS.iter().any(|a| self.is_match(a))
    }
}

impl<'a> fmt::Display for Action<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> Serialize for Action<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if !self.is_valid() {
            return Err(S::Error::custom(format!("invalid action '{}'", &self)));
        }
        serializer.serialize_str(&self)
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

// Set of actions.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct ActionSet<'a>(HashSet<Action<'a>>);

impl<'a> ActionSet<'a> {
    pub fn intersection(
        &'a self,
        other: &'a ActionSet<'a>,
    ) -> hash_set::Intersection<'a, Action<'a>, hash_map::RandomState> {
        self.0.intersection(&other.0)
    }

    pub fn insert(&mut self, value: Action<'a>) -> bool {
        self.0.insert(value)
    }

    pub fn contains(&self, value: &Action<'a>) -> bool {
        self.0.contains(value)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> hash_set::Iter<'_, Action<'a>> {
        self.0.iter()
    }

    pub fn is_match(&self, action: &Action) -> bool {
        self.iter().any(|a| {
            a.is_match(action) ||
            // This is a special case where GetObjectVersion
            // means GetObject is enabled implicitly. 
            (a == &GET_OBJECT_VERSION_ACTION.into() && action == &GET_OBJECT_ACTION.into())
        })
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        for a in self.iter() {
            if !a.is_valid() {
                bail!("invalid action '{}'", a);
            }
        }
        Ok(())
    }

    pub fn validate_admin(&self) -> anyhow::Result<()> {
        for a in self.iter() {
            if !AdminAction::from(a).is_valid() {
                bail!("invalid action '{}'", a);
            }
        }
        Ok(())
    }
}

impl<'a> crate::bucket::policy::ToVec<Action<'a>> for ActionSet<'a> {
    fn to_vec(&self) -> Vec<Action<'a>> {
        self.0.iter().cloned().collect()
    }
}

impl<'a> Default for ActionSet<'a> {
    fn default() -> Self {
        ActionSet(HashSet::new())
    }
}

impl<'a> fmt::Display for ActionSet<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut actions: Vec<&Action> = self.0.iter().collect();
        actions.sort_unstable();
        write!(f, "[")?;
        if !actions.is_empty() {
            let last = actions.len() - 1;
            for &a in &actions[..last] {
                write!(f, "{},", a.0)?;
            }
            write!(f, "{}", actions[last].0)?;
        }
        write!(f, "]")
    }
}

impl<'a> Serialize for ActionSet<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if self.is_empty() {
            return Err(S::Error::custom("empty actions"));
        }
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for e in &self.0 {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

impl<'de, 'a> Deserialize<'de> for ActionSet<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ActionSetVisitor;
        impl<'de> Visitor<'de> for ActionSetVisitor {
            type Value = ActionSet<'static>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an action array")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let action = SUPPORTED_ACTIONS
                    .iter()
                    .find(|&a| a.0 == v)
                    .cloned()
                    .ok_or(E::custom(format!("invalid action '{}'", v)))?;
                Ok(ActionSet(HashSet::from([action])))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                use serde::de::Error;
                let mut set = ActionSet(HashSet::new());
                while let Some(v) = seq.next_element()? {
                    if !set.contains(&v) {
                        set.insert(v);
                    }
                }
                if set.is_empty() {
                    return Err(A::Error::custom("empty actions"));
                }
                Ok(set)
            }
        }

        deserializer.deserialize_any(ActionSetVisitor)
    }
}

#[macro_export]
macro_rules! iam_actionset {
    ($($e:expr),*) => {{
        use crate::iam::policy::ActionSet;
        let mut set = ActionSet::default();
        $(
            set.insert($e.into());
        )*
        set
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bucket::policy::ToVec;
    use crate::utils::assert::*;

    #[test]
    fn test_action_is_object_action() {
        let cases = [
            (ABORT_MULTIPART_UPLOAD_ACTION, true),
            (DELETE_OBJECT_ACTION, true),
            (GET_OBJECT_ACTION, true),
            (LIST_MULTIPART_UPLOAD_PARTS_ACTION, true),
            (PUT_OBJECT_ACTION, true),
            (CREATE_BUCKET_ACTION, false),
        ];

        for (action, expected_result) in cases {
            let action: Action = action.into();
            let result = action.is_object_action();

            assert_eq!(
                result, expected_result,
                "action: {}, expected: {}, got: {}",
                action, expected_result, result
            );
        }
    }

    #[test]
    fn test_action_is_valid() {
        let cases = [
            (PUT_OBJECT_ACTION, true),
            (ABORT_MULTIPART_UPLOAD_ACTION, true),
            ("foo", false),
        ];

        for (action, expected_result) in cases {
            let action: Action = action.into();
            let result = action.is_valid();

            assert_eq!(
                result, expected_result,
                "action: {}, expected: {}, got: {}",
                action, expected_result, result
            );
        }
    }

    #[test]
    fn test_action_set_add() {
        let cases = [
            (
                iam_actionset!(),
                PUT_OBJECT_ACTION,
                iam_actionset!(PUT_OBJECT_ACTION),
            ),
            (
                iam_actionset!(PUT_OBJECT_ACTION),
                PUT_OBJECT_ACTION,
                iam_actionset!(PUT_OBJECT_ACTION),
            ),
        ];

        for (mut set, action_to_add, expected_result) in cases {
            let _result = set.insert(action_to_add.into());

            assert_eq!(
                set, expected_result,
                "set: {}, expected: {}, got: {}",
                set, expected_result, set
            );
        }
    }

    #[test]
    fn test_action_set_matches() {
        let cases = [
            (
                iam_actionset!(ALL_ACTIONS),
                ABORT_MULTIPART_UPLOAD_ACTION,
                true,
            ),
            (iam_actionset!(PUT_OBJECT_ACTION), PUT_OBJECT_ACTION, true),
            (
                iam_actionset!(PUT_OBJECT_ACTION, GET_OBJECT_ACTION),
                PUT_OBJECT_ACTION,
                true,
            ),
            (
                iam_actionset!(PUT_OBJECT_ACTION, GET_OBJECT_ACTION),
                ABORT_MULTIPART_UPLOAD_ACTION,
                false,
            ),
        ];

        for (set, action, expected_result) in cases {
            let action: Action = action.into();
            let result = set.is_match(&action.into());

            assert_eq!(
                result, expected_result,
                "set: {}, expected: {}, got: {}",
                set, expected_result, result
            );
        }
    }

    #[test]
    fn test_action_set_intersection() {
        let cases = [
            (
                iam_actionset!(),
                iam_actionset!(PUT_OBJECT_ACTION),
                iam_actionset!(),
            ),
            (
                iam_actionset!(PUT_OBJECT_ACTION),
                iam_actionset!(),
                iam_actionset!(),
            ),
            (
                iam_actionset!(PUT_OBJECT_ACTION),
                iam_actionset!(PUT_OBJECT_ACTION, GET_OBJECT_ACTION),
                iam_actionset!(PUT_OBJECT_ACTION),
            ),
        ];

        for (set, set_to_intersect, expected_result) in cases {
            let result = ActionSet(set.intersection(&set_to_intersect).cloned().collect());

            assert_eq!(result, expected_result);
        }
    }

    #[test]
    fn test_action_set_serialize_json() {
        let cases = [
            (
                iam_actionset!(PUT_OBJECT_ACTION),
                r#"["s3:PutObject"]"#,
                false,
            ),
            (iam_actionset!(), "", true),
        ];

        for (set, expected_result, expect_err) in cases {
            let result = serde_json::to_string(&set);

            match result {
                Ok(result) => assert_eq!(result, expected_result),
                Err(_) => assert!(expect_err),
            }
        }
    }

    #[test]
    fn test_action_set_deserialize_json() {
        let cases = [
            (
                r#""s3:PutObject""#,
                iam_actionset!(PUT_OBJECT_ACTION),
                false,
                false,
            ),
            (
                r#"["s3:PutObject"]"#,
                iam_actionset!(PUT_OBJECT_ACTION),
                false,
                false,
            ),
            (
                r#"["s3:PutObject", "s3:GetObject"]"#,
                iam_actionset!(PUT_OBJECT_ACTION, GET_OBJECT_ACTION),
                false,
                false,
            ),
            (
                r#"["s3:PutObject", "s3:GetObject", "s3:PutObject"]"#,
                iam_actionset!(PUT_OBJECT_ACTION, GET_OBJECT_ACTION),
                false,
                false,
            ),
            (r#"[]"#, iam_actionset!(), true, false),
            (r#""foo""#, iam_actionset!(), true, false),
            (r#"["s3:PutObject", "foo"]"#, iam_actionset!(), true, false),
        ];

        for (data, expected_result, expect_deserialize_err, expect_validate_err) in cases {
            let result = serde_json::from_str::<ActionSet>(data);

            match result {
                Ok(result) => {
                    if expect_validate_err {
                        assert_err!(result.validate());
                    } else {
                        assert_ok!(result.validate());
                    }
                    assert_eq!(result, expected_result);
                }
                Err(_) => assert!(expect_deserialize_err),
            }
        }
    }

    #[test]
    fn test_set_to_vec() {
        let cases = [
            (
                iam_actionset!(PUT_OBJECT_ACTION),
                vec![Action::from(PUT_OBJECT_ACTION)],
            ),
            (iam_actionset!(), vec![]),
        ];

        for (set, expected_result) in cases {
            let result = set.to_vec();

            assert_eq!(result, expected_result);
        }
    }
}
