use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use strum::{Display, EnumCount};

use super::*;
use crate::globals::{self, GLOBALS};
use crate::utils::AtomicExt;

#[derive(FromPrimitive, EnumCount, Display, Copy, Clone)]
#[repr(u8)]
enum StorageMetric {
    MakeVolBulk,
    MakeVol,
    ListVols,
    StatVol,
    DeleteVol,
    WalkDir,
    ListDir,
    ReadFile,
    AppendFile,
    CreateFile,
    ReadFileStream,
    RenameFile,
    RenameData,
    CheckParts,
    CheckFile,
    Delete,
    DeleteVersions,
    VerifyFile,
    WriteAll,
    DeleteVersion,
    WriteMetadata,
    UpdateMetadata,
    ReadVersion,
    ReadAll,
}

pub(super) struct XlStorageWithCheck {
    storage: XlStorage,
    disk_id: Option<String>,
    api_latencies:
        [Arc<RwLock<(ta::indicators::ExponentialMovingAverage, f64)>>; StorageMetric::COUNT],
    api_calls: [AtomicU64; StorageMetric::COUNT],
}

impl XlStorageWithCheck {
    pub fn new(storage: XlStorage) -> Self {
        Self {
            storage,
            disk_id: None,
            api_latencies: [0u8; StorageMetric::COUNT].map(|_| {
                Arc::new(RwLock::new((
                    ta::indicators::ExponentialMovingAverage::new(30).unwrap(),
                    0f64,
                )))
            }),
            api_calls: [0u8; StorageMetric::COUNT].map(|_| AtomicU64::new(0)),
        }
    }

    fn get_metrics(&self) -> crate::storage::DiskMetrics {
        crate::storage::DiskMetrics {
            api_latencies: self
                .api_latencies
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let v = l.read().unwrap().1;
                    (
                        StorageMetric::from_usize(i).unwrap().to_string(),
                        chrono::Duration::nanoseconds(v as i64).to_string(),
                    )
                })
                .collect(),
            api_calls: self
                .api_calls
                .iter()
                .enumerate()
                .map(|(i, n)| (StorageMetric::from_usize(i).unwrap().to_string(), n.get()))
                .collect(),
        }
    }

    fn update_metrics<'a, 'b: 'a>(
        &'b self,
        m: StorageMetric,
        paths: &'a [&str],
    ) -> impl 'a + FnOnce() {
        let start_time = utils::now();
        return move || {
            use ta::Next;
            let duration = utils::now().signed_duration_since(start_time);
            self.api_calls[m as usize].inc();
            let mut latency = self.api_latencies[m as usize].write().unwrap();
            // Safety: here duration will not overflow.
            latency.1 = latency.0.next(duration.num_nanoseconds().unwrap() as f64);

            if GLOBALS.trace.subscribers_num() > 0 {
                GLOBALS.trace.publish(crate::admin::TraceInfo {
                    trace_type: crate::admin::TraceType::Storage,
                    node_name: GLOBALS.local_node_name.guard().to_owned(),
                    fn_name: format!("storage.{}", m),
                    time: start_time,
                    storage_stats: Some(crate::admin::TraceStorageStats {
                        path: crate::object::path_join(paths),
                        duration: duration.to_std().unwrap(),
                    }),
                    ..Default::default()
                });
            }
        };
    }
}
