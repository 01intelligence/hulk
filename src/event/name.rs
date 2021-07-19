use std::fmt;

use derivative::Derivative;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Serialize, Deserialize, Derivative, Clone, Eq, PartialEq, Hash)]
#[derivative(Default)]
pub enum Name {
    #[derivative(Default)]
    Unspecified,
    ObjectAccessedAll,
    ObjectAccessedGet,
    ObjectAccessedGetRetention,
    ObjectAccessedGetLegalHold,
    ObjectAccessedHead,
    ObjectCreatedAll,
    ObjectCreatedCompleteMultipartUpload,
    ObjectCreatedCopy,
    ObjectCreatedPost,
    ObjectCreatedPut,
    ObjectCreatedPutRetention,
    ObjectCreatedPutLegalHold,
    ObjectCreatedPutTagging,
    ObjectCreatedDeleteTagging,
    ObjectRemovedAll,
    ObjectRemovedDelete,
    ObjectRemovedDeleteMarkerCreated,
    BucketCreated,
    BucketRemoved,
    ObjectReplicationAll,
    ObjectReplicationFailed,
    ObjectReplicationComplete,
    ObjectReplicationMissedThreshold,
    ObjectReplicationReplicatedAfterThreshold,
    ObjectReplicationNotTracked,
    ObjectRestorePostInitiated,
    ObjectRestorePostCompleted,
    ObjectRestorePostAll,
    ObjectTransitionAll,
    ObjectTransitionFailed,
    ObjectTransitionComplete,
}

impl Name {
    pub fn parse(s: &str) -> anyhow::Result<Name> {
        use Name::*;
        let n = match s {
            "s3:BucketCreated:*" => BucketCreated,
            "s3:BucketRemoved:*" => BucketRemoved,
            "s3:ObjectAccessed:*" => ObjectAccessedAll,
            "s3:ObjectAccessed:Get" => ObjectAccessedGet,
            "s3:ObjectAccessed:GetRetention" => ObjectAccessedGetRetention,
            "s3:ObjectAccessed:GetLegalHold" => ObjectAccessedGetLegalHold,
            "s3:ObjectAccessed:Head" => ObjectAccessedHead,
            "s3:ObjectCreated:*" => ObjectCreatedAll,
            "s3:ObjectCreated:CompleteMultipartUpload" => ObjectCreatedCompleteMultipartUpload,
            "s3:ObjectCreated:Copy" => ObjectCreatedCopy,
            "s3:ObjectCreated:Post" => ObjectCreatedPost,
            "s3:ObjectCreated:Put" => ObjectCreatedPut,
            "s3:ObjectCreated:PutRetention" => ObjectCreatedPutRetention,
            "s3:ObjectCreated:PutLegalHold" => ObjectCreatedPutLegalHold,
            "s3:ObjectCreated:PutTagging" => ObjectCreatedPutTagging,
            "s3:ObjectCreated:DeleteTagging" => ObjectCreatedDeleteTagging,
            "s3:ObjectRemoved:*" => ObjectRemovedAll,
            "s3:ObjectRemoved:Delete" => ObjectRemovedDelete,
            "s3:ObjectRemoved:DeleteMarkerCreated" => ObjectRemovedDeleteMarkerCreated,
            "s3:Replication:*" => ObjectReplicationAll,
            "s3:Replication:OperationFailedReplication" => ObjectReplicationFailed,
            "s3:Replication:OperationCompletedReplication" => ObjectReplicationComplete,
            "s3:Replication:OperationMissedThreshold" => ObjectReplicationMissedThreshold,
            "s3:Replication:OperationReplicatedAfterThreshold" => {
                ObjectReplicationReplicatedAfterThreshold
            }
            "s3:Replication:OperationNotTracked" => ObjectReplicationNotTracked,
            "s3:ObjectRestore:*" => ObjectRestorePostAll,
            "s3:ObjectRestore:Post" => ObjectRestorePostInitiated,
            "s3:ObjectRestore:Completed" => ObjectRestorePostCompleted,
            "s3:ObjectTransition:Failed" => ObjectTransitionFailed,
            "s3:ObjectTransition:Complete" => ObjectTransitionComplete,
            "s3:ObjectTransition:*" => ObjectTransitionAll,
            s => {
                return Err(EventError::InvalidEventName(s.to_owned()).into());
            }
        };
        Ok(n)
    }

    pub fn expand(&self) -> Vec<Name> {
        use Name::*;
        match self {
            &ObjectAccessedAll => vec![
                ObjectAccessedGet,
                ObjectAccessedHead,
                ObjectAccessedGetRetention,
                ObjectAccessedGetLegalHold,
            ],
            &ObjectCreatedAll => vec![
                ObjectCreatedCompleteMultipartUpload,
                ObjectCreatedCopy,
                ObjectCreatedPost,
                ObjectCreatedPut,
                ObjectCreatedPutRetention,
                ObjectCreatedPutLegalHold,
                ObjectCreatedPutTagging,
                ObjectCreatedDeleteTagging,
            ],
            &ObjectRemovedAll => vec![ObjectRemovedDelete, ObjectRemovedDeleteMarkerCreated],
            &ObjectReplicationAll => vec![
                ObjectReplicationFailed,
                ObjectReplicationComplete,
                ObjectReplicationNotTracked,
                ObjectReplicationMissedThreshold,
                ObjectReplicationReplicatedAfterThreshold,
            ],
            &ObjectRestorePostAll => vec![ObjectRestorePostInitiated, ObjectRestorePostCompleted],
            &ObjectTransitionAll => vec![ObjectTransitionFailed, ObjectTransitionComplete],
            n => vec![n.clone()],
        }
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Name::*;
        let s = match self {
            &BucketCreated => "s3:BucketCreated:*",
            &BucketRemoved => "s3:BucketRemoved:*",
            &ObjectAccessedAll => "s3:ObjectAccessed:*",
            &ObjectAccessedGet => "s3:ObjectAccessed:Get",
            &ObjectAccessedGetRetention => "s3:ObjectAccessed:GetRetention",
            &ObjectAccessedGetLegalHold => "s3:ObjectAccessed:GetLegalHold",
            &ObjectAccessedHead => "s3:ObjectAccessed:Head",
            &ObjectCreatedAll => "s3:ObjectCreated:*",
            &ObjectCreatedCompleteMultipartUpload => "s3:ObjectCreated:CompleteMultipartUpload",
            &ObjectCreatedCopy => "s3:ObjectCreated:Copy",
            &ObjectCreatedPost => "s3:ObjectCreated:Post",
            &ObjectCreatedPut => "s3:ObjectCreated:Put",
            &ObjectCreatedPutTagging => "s3:ObjectCreated:PutTagging",
            &ObjectCreatedDeleteTagging => "s3:ObjectCreated:DeleteTagging",
            &ObjectCreatedPutRetention => "s3:ObjectCreated:PutRetention",
            &ObjectCreatedPutLegalHold => "s3:ObjectCreated:PutLegalHold",
            &ObjectRemovedAll => "s3:ObjectRemoved:*",
            &ObjectRemovedDelete => "s3:ObjectRemoved:Delete",
            &ObjectRemovedDeleteMarkerCreated => "s3:ObjectRemoved:DeleteMarkerCreated",
            &ObjectReplicationAll => "s3:Replication:*",
            &ObjectReplicationFailed => "s3:Replication:OperationFailedReplication",
            &ObjectReplicationComplete => "s3:Replication:OperationCompletedReplication",
            &ObjectReplicationNotTracked => "s3:Replication:OperationNotTracked",
            &ObjectReplicationMissedThreshold => "s3:Replication:OperationMissedThreshold",
            &ObjectReplicationReplicatedAfterThreshold => {
                "s3:Replication:OperationReplicatedAfterThreshold"
            }
            &ObjectRestorePostAll => "s3:ObjectRestore:*",
            &ObjectRestorePostInitiated => "s3:ObjectRestore:Post",
            &ObjectRestorePostCompleted => "s3:ObjectRestore:Completed",
            &ObjectTransitionAll => "s3:ObjectTransition:*",
            &ObjectTransitionFailed => "s3:ObjectTransition:Failed",
            &ObjectTransitionComplete => "s3:ObjectTransition:Complete",
            &Unspecified => "",
        };
        write!(f, "{}", s)
    }
}
