use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use super::*;

// Represents access key who caused the event.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub principal_id: String,
}

// Represents bucket metadata of the event.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub name: String,
    pub owner_identity: Identity,
    pub arn: String,
}

// Represents object metadata of the event.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Object {
    pub key: String,
    pub size: u64,
    #[serde(rename = "eTag")]
    pub etag: String,
    pub content_type: String,
    pub user_metadata: HashMap<String, String>,
    pub version_id: String,
    pub sequencer: String,
}

// Represents event metadata.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    #[serde(rename = "s3SchemaVersion")]
    pub schema_version: String,
    pub configuration_id: String,
    pub bucket: Bucket,
    pub object: Object,
}

// Represents client information who triggered the event.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub host: String,
    pub port: String,
    pub user_agent: String,
}

// Represents event notification information defined in
// http://docs.aws.amazon.com/AmazonS3/latest/dev/notification-content-structure.html.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub event_version: String,
    pub event_source: String,
    pub aws_region: String,
    pub event_time: String,
    pub event_name: Name,
    pub user_identity: Identity,
    pub request_parameters: HashMap<String, String>,
    pub response_elements: HashMap<String, String>,
    pub s3: Metadata,
    pub source: Source,
}

// Represents event information for some event targets.
pub struct Log {
    pub event_name: Name,
    pub key: String,
    pub records: Vec<Event>,
}
