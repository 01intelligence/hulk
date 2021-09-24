use std::collections::HashMap;

use async_trait::async_trait;
use tonic::{Request, Response, Status};

use super::*;
use crate::endpoint::EndpointServerPools;
use crate::proto;
use crate::proto::{DiskInfo, Empty, Version};
use crate::xl_storage::XlStorage;

const STATUS_DISK_STALE: &str = "disk stale";
const STATUS_DISK_PATH_INVALID: &str = "disk path invalid";

pub struct StorageService {
    stores: HashMap<String, XlStorage>,
}

impl StorageService {
    pub async fn new(endpoint_server_pools: EndpointServerPools) -> StorageService {
        let mut handles = Vec::new();
        for ep in endpoint_server_pools.iter() {
            for endpoint in ep.endpoints.iter() {
                if !endpoint.is_local() {
                    // Only process endpoints which is belong to myself.
                    continue;
                }
                let endpoint = endpoint.clone();

                handles.push(tokio::spawn(async move {
                    match XlStorage::new(endpoint).await {
                        Ok(store) => Some(store),
                        Err(_) => {
                            // TODO: log
                            None
                        }
                    }
                }));
            }
        }

        let mut stores = HashMap::new();
        for r in futures_util::future::join_all(handles).await {
            let r = r.unwrap(); // no task should panic
            if let Some(store) = r {
                stores.insert(store.endpoint().path().to_owned(), store);
            }
        }
        StorageService { stores }
    }

    async fn prepare<T>(&self, req: &Request<T>) -> Result<&XlStorage, Status> {
        let meta = req.metadata();

        let disk_path = meta
            .get_str("disk-path")
            .ok_or_else(|| Status::invalid_argument(STATUS_DISK_PATH_INVALID))?;

        let store = self
            .stores
            .get(disk_path)
            .ok_or_else(|| Status::invalid_argument(STATUS_DISK_PATH_INVALID))?;

        match meta.get_str("disk-id") {
            None => {} // Allow for newly coming up peer.
            Some(req_disk_id) => {
                let disk_id = store
                    .get_disk_id()
                    .await
                    .map_err(|err| Status::unavailable(err.to_string()))?;
                if req_disk_id != disk_id {
                    return Err(Status::failed_precondition(STATUS_DISK_STALE));
                }
            }
        }
        Ok(store)
    }
}

#[async_trait]
impl proto::StorageService for StorageService {
    async fn version(&self, req: Request<Empty>) -> Result<Response<Version>, Status> {
        todo!()
    }

    async fn health(&self, req: Request<Empty>) -> Result<Response<Empty>, Status> {
        let _ = self.prepare(&req).await?;
        Ok(Response::new(Empty {}))
    }

    async fn disk_info(&self, req: Request<Empty>) -> Result<Response<DiskInfo>, Status> {
        let store = self.prepare(&req).await?;
        let disk_info = store
            .disk_info()
            .await
            .map_err(|err| Status::internal(err.to_string()))?;
        Ok(Response::new(DiskInfo {
            encoded: rmp_serde::to_vec(&disk_info).unwrap(),
        }))
    }
}
