use std::collections::HashMap;

use async_trait::async_trait;
use tonic::{Request, Response, Status};

use crate::endpoint::EndpointServerPools;
use crate::proto;
use crate::proto::storage::HealthReq;
use crate::proto::{Empty, Version};
use crate::xl_storage::XlStorage;

pub struct StorageService {
    stores: HashMap<String, XlStorage>,
}

impl StorageService {
    pub async fn new(endpoint_server_pools: &EndpointServerPools) -> StorageService {
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

    fn validate<T>(&self, req: &Request<T>) -> Result<(), Status> {
        todo!()
    }
}

#[async_trait]
impl proto::StorageService for StorageService {
    async fn version(&self, req: Request<Empty>) -> Result<Response<Version>, Status> {
        todo!()
    }

    async fn health(&self, req: Request<HealthReq>) -> Result<Response<Empty>, Status> {
        self.validate(&req)?;
        todo!()
    }
}
