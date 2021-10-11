use std::collections::HashMap;

use actix_web::web::Query;

use crate::globals::{ReadWriteGuard, GLOBALS};
use crate::logger::ReqInfo;
use crate::utils;
use crate::utils::DateTimeFormatExt;

#[derive(Clone)]
pub struct Audit {
    pub req_info: ReqInfo,
    pub request_claims: crate::jwt::MapClaims,
    pub filter_keys: Vec<String>,
}

impl Audit {
    pub fn audit(&self, trace: &crate::admin::TraceInfo) {
        let trace_req_info = trace.req_info.as_ref().unwrap();
        let trace_resp_info = trace.resp_info.as_ref().unwrap();
        let trace_call_info = trace.call_stats.as_ref().unwrap();

        let req_query = if let Ok(query) =
            Query::<HashMap<String, String>>::from_query(&trace_req_info.raw_query)
        {
            query.into_inner()
        } else {
            Default::default()
        };

        let entry = super::entry::audit::Entry {
            version: super::entry::audit::VERSION.to_owned(),
            deployment_id: GLOBALS.deployment_id.guard().clone(),
            time: utils::now().rfc3339_nano(),
            trigger: "external-request".to_string(),
            api: super::entry::audit::Api {
                name: self.req_info.api.clone(), // TODO
                bucket: self.req_info.bucket_name.clone(),
                object: self.req_info.object_name.clone(),
                status: trace_resp_info.status_code.to_string(),
                status_code: trace_resp_info.status_code.as_u16(),
                time_to_first_byte: format!("{}ns", trace_call_info.time_to_first_byte.as_nanos()),
                time_to_response: format!("{}ns", trace_call_info.latency.as_nanos()),
            },
            remote_host: self.req_info.remote_host.clone(),
            request_id: self.req_info.request_id.clone(),
            user_agent: self.req_info.user_agent.clone(),
            request_claims: Default::default(),
            request_query: req_query,
            request_header: Default::default(),
            response_header: Default::default(),
            tags: self.req_info.get_tags_map(),
        };
    }
}
