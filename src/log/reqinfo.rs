use std::collections::HashMap;

use opentelemetry::Context;

use crate::log::entry::Value;

lazy_static::lazy_static! {
    static ref NOOP_REQ_INFO: ReqInfo = ReqInfo::default();
}

#[derive(Default, Debug)]
pub struct ReqInfo {
    pub remote_host: String,   // Client Host/IP
    pub host: String,          // Node Host/IP
    pub user_agent: String,    // User Agent
    pub deployment_id: String, // x-minio-deployment-id
    pub request_id: String,    // x-amz-request-id
    pub api: String,           // API name - GetObject PutObject NewMultipartUpload etc.
    pub bucket_name: String,   // Bucket name
    pub object_name: String,   // Object name
    pub access_key: String,    // Access Key
    tags: Vec<KeyValue>,       // Any additional info not accommodated by above fields
}

#[derive(Default, Debug)]
pub struct KeyValue {
    key: String,
    val: Value,
}

impl ReqInfo {
    pub fn new(
        remote_host: String,
        user_agent: String,
        deployment_id: String,
        request_id: String,
        api: String,
        bucket: String,
        object: String,
    ) -> ReqInfo {
        ReqInfo {
            remote_host,
            host: "".to_string(),
            user_agent,
            deployment_id,
            request_id,
            api,
            bucket_name: bucket,
            object_name: object,
            access_key: "".to_string(),
            tags: vec![],
        }
    }

    pub fn append_tag(&mut self, key: String, val: Value) {
        self.tags.push(KeyValue { key, val });
    }

    pub fn set_tag(&mut self, key: String, val: Value) {
        // Search of tag key already exists in tags
        if let Some(kv) = self.tags.iter_mut().find(|kv| kv.key == key) {
            kv.val = val;
        } else {
            // Append to the end of tags list
            self.append_tag(key, val);
        }
    }

    pub fn get_tags(&self) -> &Vec<KeyValue> {
        &self.tags
    }

    pub fn get_tags_map(&self) -> HashMap<String, Value> {
        let mut map = HashMap::with_capacity(self.tags.len());
        for kv in &self.tags {
            map.insert(kv.key.clone(), kv.val.clone());
        }
        map
    }
}

pub trait ReqInfoContextExt {
    fn with_req_info(&self, req: ReqInfo) -> Self;
    fn current_with_req_info(req: ReqInfo) -> Self;
    fn req_info(&self) -> &'_ ReqInfo;
}

impl ReqInfoContextExt for Context {
    fn with_req_info(&self, req: ReqInfo) -> Self {
        self.with_value(req)
    }

    fn current_with_req_info(req: ReqInfo) -> Self {
        Context::current_with_value(req)
    }

    fn req_info(&self) -> &'_ ReqInfo {
        self.get::<ReqInfo>().unwrap_or(&*NOOP_REQ_INFO)
    }
}
