use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::router;
use crate::utils::AtomicExt;

// Holds statistics information about
// a given API in the requests.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HttpApiStats {
    api_stats: dashmap::DashMap<Cow<'static, str>, u64>,
}

impl HttpApiStats {
    pub fn inc_guard<T: Into<Cow<'static, str>>>(&self, api: T) -> HttpApiStatsGuard<'_> {
        let api = api.into();
        let guard = HttpApiStatsGuard {
            inner: self,
            api: Some(api.clone()),
        };
        self.inc(api);
        guard
    }

    pub fn inc<T: Into<Cow<'static, str>>>(&self, api: T) {
        let mut e = self.api_stats.entry(api.into()).or_default();
        *e.value_mut() += 1;
    }

    pub fn dec<T: Into<Cow<'static, str>>>(&self, api: T) {
        let api = api.into();
        if let Some(mut e) = self.api_stats.get_mut(&api) {
            let val = e.value_mut();
            if *val > 0 {
                *val -= 1;
                if *val == 0 {
                    let _ = self.api_stats.remove(&api);
                }
            }
        }
    }

    pub fn view(&self) -> HashMap<Cow<'static, str>, u64> {
        self.api_stats
            .iter()
            .map(|e| (e.key().to_owned(), *e.value()))
            .collect()
    }
}

pub struct HttpApiStatsGuard<'a> {
    inner: &'a HttpApiStats,
    api: Option<Cow<'static, str>>,
}

impl<'a> HttpApiStatsGuard<'a> {
    pub fn api(&self) -> Cow<'static, str> {
        self.api.as_ref().unwrap().clone()
    }
}

impl<'a> Drop for HttpApiStatsGuard<'a> {
    fn drop(&mut self) {
        self.inner.dec(self.api.take().unwrap());
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

pub struct HttpStatsAddRequestsGuard<'a> {
    stats: &'a HttpStats,
}

impl<'a> Drop for HttpStatsAddRequestsGuard<'a> {
    fn drop(&mut self) {
        self.stats.s3_requests_in_queue.dec();
    }
}

impl HttpStats {
    pub fn add_requests_in_queue(&self) -> HttpStatsAddRequestsGuard<'_> {
        self.s3_requests_in_queue.inc();
        HttpStatsAddRequestsGuard { stats: self }
    }

    pub fn update_stats<S: Into<Cow<'static, str>>>(
        &self,
        api: S,
        status: StatusCode,
        uri_path: &str,
    ) {
        let api = api.into();
        let success = status >= StatusCode::OK && status < StatusCode::MULTIPLE_CHOICES;

        if !uri_path.ends_with(router::PROMETHEUS_METRICS_V2_CLUSTER_PATH)
            && !uri_path.ends_with(router::PROMETHEUS_METRICS_V2_NODE_PATH)
        {
            self.total_s3_requests.inc(api.clone());
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
