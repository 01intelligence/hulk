use serde::{Deserialize, Serialize};
use strum::Display;

use super::*;
use crate::prelude::*;

pub const TRANSITION_STATUS: &str = "transition-status";
pub const TRANSITIONED_OBJECT_NAME: &str = "transitioned-object";
pub const TRANSITIONED_VERSION_ID: &str = "transitioned-versionID";
pub const TRANSITION_TIER: &str = "transition-tier";

#[derive(Display)]
pub enum TransitionStatus {
    #[strum(serialize = "complete")]
    Complete,
    #[strum(serialize = "pending")]
    Pending,
}

#[derive(Serialize, Deserialize)]
pub enum RestoreRequestType {
    #[serde(rename = "SELECT")]
    Select,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Grantee {
    /// <p>Screen name of the grantee.</p>
    pub display_name: Option<String>,
    /// <p><p>Email address of the grantee.</p> <note> <p>Using email addresses to specify a grantee is only supported in the following AWS Regions: </p> <ul> <li> <p>US East (N. Virginia)</p> </li> <li> <p>US West (N. California)</p> </li> <li> <p> US West (Oregon)</p> </li> <li> <p> Asia Pacific (Singapore)</p> </li> <li> <p>Asia Pacific (Sydney)</p> </li> <li> <p>Asia Pacific (Tokyo)</p> </li> <li> <p>Europe (Ireland)</p> </li> <li> <p>South America (São Paulo)</p> </li> </ul> <p>For a list of all the Amazon S3 supported Regions and endpoints, see <a href="https://docs.aws.amazon.com/general/latest/gr/rande.html#s3_region">Regions and Endpoints</a> in the AWS General Reference.</p> </note></p>
    pub email_address: Option<String>,
    /// <p>The canonical user ID of the grantee.</p>
    pub id: Option<String>,
    /// <p>Type of grantee</p>
    pub type_: String,
    /// <p>URI of the grantee group.</p>
    pub uri: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Grant {
    /// <p>The person being granted permissions.</p>
    pub grantee: Option<Grantee>,
    /// <p>Specifies the permission given to the grantee.</p>
    pub permission: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Encryption {
    /// <p>The server-side encryption algorithm used when storing job results in Amazon S3 (for example, AES256, aws:kms).</p>
    pub encryption_type: encryption::SseAlgorithm,
    /// <p>If the encryption type is <code>aws:kms</code>, this optional value can be used to specify the encryption context for the restore results.</p>
    #[serde(rename = "KMSContext")]
    pub kms_context: Option<String>,
    /// <p>If the encryption type is <code>aws:kms</code>, this optional value specifies the ID of the symmetric customer managed AWS KMS CMK to use for encryption of job results. Amazon S3 only supports symmetric CMKs. For more information, see <a href="https://docs.aws.amazon.com/kms/latest/developerguide/symmetric-asymmetric.html">Using symmetric and asymmetric keys</a> in the <i>AWS Key Management Service Developer Guide</i>.</p>
    #[serde(rename = "KMSKeyId")]
    pub kms_key_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MetadataEntry {
    /// <p>Name of the Object.</p>
    pub name: Option<String>,
    /// <p>Value of the Object.</p>
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct S3Location {
    /// <p>A list of grants that control access to the staged results.</p>
    pub access_control_list: Option<Vec<Grant>>,
    /// <p>The name of the bucket where the restore results will be placed.</p>
    pub bucket_name: String,
    /// <p>The canned ACL to apply to the restore results.</p>
    pub canned_acl: Option<String>,
    pub encryption: Option<Encryption>,
    /// <p>The prefix that is prepended to the restore results for this request.</p>
    pub prefix: String,
    /// <p>The class of storage used to store the restore results.</p>
    pub storage_class: Option<String>,
    /// <p>The tag-set that is applied to the restore results.</p>
    pub tagging: Option<crate::tags::Tagging>,
    /// <p>A list of metadata to store with the restore results in S3.</p>
    pub user_metadata: Option<Vec<MetadataEntry>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OutputLocation {
    /// <p>Describes an S3 location that will receive the results of the restore request.</p>
    pub s3: Option<S3Location>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GlacierJobParameters {
    /// <p>Retrieval tier at which the restore will be processed.</p>
    pub tier: String,
}

// TODO: attribute xmlns="http://s3.amazonaws.com/doc/2006-03-01/"
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct RestoreRequest {
    /// <p>Lifetime of the active copy in days. Do not use with restores that specify <code>OutputLocation</code>.</p> <p>The Days element is required for regular restores, and must not be provided for select requests.</p>
    pub days: Option<i64>,
    /// <p>The optional description for the job.</p>
    pub description: Option<String>,
    /// <p>S3 Glacier related parameters pertaining to this job. Do not use with restores that specify <code>OutputLocation</code>.</p>
    pub glacier_job_parameters: Option<GlacierJobParameters>,
    /// <p>Describes the location where the restore job's output is stored.</p>
    pub output_location: Option<OutputLocation>,
    /// <p>Describes the parameters for Select job types.</p>
    pub select_parameters: Option<crate::s3select::SelectParameters>,
    /// <p>Retrieval tier at which the restore will be processed.</p>
    pub tier: Option<String>,
    /// <p>Type of restore request.</p>
    #[serde(rename = "Type")]
    pub type_: Option<RestoreRequestType>,
}

pub fn is_restored_object_on_disk(meta: &HashMap<String, String>) -> bool {
    // TODO
    false
}
