use tonic::transport::Channel;

use super::*;
use crate::proto::storage::DiskInfo;
use crate::proto::{storage, StorageServiceClient};

pub struct StorageClient {
    endpoint: crate::endpoint::Endpoint,
    client: StorageServiceClient<HealthCheckChannel>,
    disk_id: String,

    // Indexes, will be -1 until assigned a set.
    pool_index: isize,
    set_index: isize,
    disk_index: isize,
}

impl StorageClient {
    pub fn new(uri: &str) -> anyhow::Result<Self> {
        let client = StorageServiceClient::new(get_inter_node_client_builder().build(uri)?);
        todo!()
    }
}
