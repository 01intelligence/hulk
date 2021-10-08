use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;

use derivative::Derivative;
use serde::{Deserialize, Serialize};
use slog::{Key, Record, SerdeValue, Serializer, KV};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Entry {
    Log(log::Entry),
    Audit(audit::Entry),
}

pub mod log {
    use super::*;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Entry {
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub deployment_id: String,
        pub level: String,
        pub kind: ErrKind,
        pub time: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub api: Option<Api>,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub remote_host: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub host: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub request_id: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub user_agent: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub message: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub error: Option<Trace>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Api {
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub name: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub args: Option<Args>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Trace {
        #[serde(skip_serializing_if = "String::is_empty")]
        pub message: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub source: Vec<String>,
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        pub variables: HashMap<String, Value>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub bucket: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub object: String,
        #[serde(skip_serializing_if = "HashMap::is_empty", default)]
        pub metadata: HashMap<String, String>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Derivative)]
    #[derivative(Default)]
    pub enum Value {
        #[derivative(Default)]
        None,
        String(Cow<'static, str>),
    }

    impl slog::Value for Entry {
        fn serialize(
            &self,
            record: &Record,
            key: Key,
            serializer: &mut Serializer,
        ) -> slog::Result {
            serializer.emit_serde(key, self)
        }
    }

    impl SerdeValue for Entry {
        fn as_serde(&self) -> &typetag::erased_serde::Serialize {
            self
        }

        fn to_sendable(&self) -> Box<dyn SerdeValue + Send + 'static> {
            Box::new(self.clone())
        }
    }

    impl KV for Entry {
        fn serialize(&self, record: &Record, serializer: &mut Serializer) -> slog::Result {
            serializer.emit_serde("", self)
        }
    }
}

pub mod audit {
    use super::*;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Entry {
        pub version: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub deployment_id: String,
        pub time: String,
        pub trigger: String,
        pub api: Api,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub remote_host: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub request_id: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub user_agent: String,
        #[serde(skip_serializing_if = "HashMap::is_empty", default)]
        pub request_claims: HashMap<String, String>,
        #[serde(skip_serializing_if = "HashMap::is_empty", default)]
        pub request_query: HashMap<String, String>,
        #[serde(skip_serializing_if = "HashMap::is_empty", default)]
        pub request_header: HashMap<String, String>,
        #[serde(skip_serializing_if = "HashMap::is_empty", default)]
        pub response_header: HashMap<String, String>,
        #[serde(skip_serializing_if = "HashMap::is_empty", default)]
        pub tags: HashMap<String, String>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Api {
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub name: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub bucket: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub object: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub status: String,
        pub status_code: u16,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub time_to_first_byte: String,
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub time_to_response: String,
    }

    impl slog::Value for Entry {
        fn serialize(
            &self,
            record: &Record,
            key: Key,
            serializer: &mut Serializer,
        ) -> slog::Result {
            serializer.emit_serde(key, self)
        }
    }

    impl SerdeValue for Entry {
        fn as_serde(&self) -> &typetag::erased_serde::Serialize {
            self
        }

        fn to_sendable(&self) -> Box<dyn SerdeValue + Send + 'static> {
            Box::new(self.clone())
        }
    }

    impl KV for Entry {
        fn serialize(&self, record: &Record, serializer: &mut Serializer) -> slog::Result {
            serializer.emit_serde("", self)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, strum::ToString, Debug)]
pub enum ErrKind {
    System,
    Application,
    All,
}
