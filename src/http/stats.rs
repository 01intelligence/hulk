use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::router;

// Holds statistics information about
// a given API in the requests.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HttpApiStats {
    api_stats: dashmap::DashMap<String, u64>,
}

impl HttpApiStats {
    pub fn inc(&self, api: &str) {
        let mut e = self.api_stats.entry(api.to_owned()).or_default();
        *e.value_mut() += 1;
    }

    pub fn dec(&self, api: &str) {
        let mut e = self.api_stats.entry(api.to_owned()).or_default();
        let val = e.value_mut();
        if *val > 0 {
            *val -= 1;
        }
    }

    pub fn view(&self) -> HashMap<String, u64> {
        self.api_stats
            .iter()
            .map(|e| (e.key().to_owned(), *e.value()))
            .collect()
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HttpStats {
    pub s3_requests_in_queue: AtomicU64,
    pub current_s3_requests: HttpApiStats,
    pub total_s3_requests: HttpApiStats,
    pub total_s3_errors: HttpApiStats,
    pub total_s3_canceled: HttpApiStats,
    pub total_s3_rejected_auth: AtomicU64,
    pub total_s3_rejected_time: AtomicU64,
    pub total_s3_rejected_header: AtomicU64,
    pub total_s3_rejected_invalid: AtomicU64,
}

impl HttpStats {
    pub fn update_stats(&self, api: &str, r: &ServiceResponse) {
        let status = r.response().status();
        let success = status >= StatusCode::OK && status < StatusCode::MULTIPLE_CHOICES;

        let path = r.request().uri().path();
        if !path.ends_with(router::PROMETHEUS_METRICS_V2_CLUSTER_PATH)
            && !path.ends_with(router::PROMETHEUS_METRICS_V2_NODE_PATH)
        {
            self.total_s3_requests.inc(api);
            if !success {
                match status.as_u16() {
                    0 | 499 => {
                        self.total_s3_canceled.inc(api);
                    }
                    _ => {
                        self.total_s3_errors.inc(api);
                    }
                }
            }
        }

        // Increment the prometheus http request response histogram with appropriate label
        // TODO
    }
}
