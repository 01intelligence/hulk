use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;

use derivative::Derivative;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Entry {
    #[serde(rename = "deploymentid", skip_serializing_if = "String::is_empty")]
    pub deployment_id: String,
    pub level: String,
    #[serde(rename = "errKind")]
    pub log_kind: String,
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<Api>,
    #[serde(rename = "remotehost", skip_serializing_if = "String::is_empty")]
    pub remote_host: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub host: String,
    #[serde(rename = "requestID", skip_serializing_if = "String::is_empty")]
    pub request_id: String,
    #[serde(rename = "userAgent", skip_serializing_if = "String::is_empty")]
    pub user_agent: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(rename = "error", skip_serializing_if = "Option::is_none")]
    pub trace: Option<Trace>,
}

#[derive(Serialize, Debug)]
pub struct Api {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Args>,
}

#[derive(Serialize, Debug)]
pub struct Trace {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub source: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, Value>,
}

#[derive(Serialize, Debug)]
pub struct Args {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bucket: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub object: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

#[derive(Serialize, Debug, Clone, Derivative)]
#[derivative(Default)]
pub enum Value {
    #[derivative(Default)]
    None,
    String(Cow<'static, str>),
}

#[derive(strum::ToString, Debug)]
pub enum ErrKind {
    Hulk,
    Application,
    All,
}
