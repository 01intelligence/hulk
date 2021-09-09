use std::sync::Arc;

use tokio::sync::RwLock;

use super::*;
use crate::endpoint::Endpoint;
use crate::proto;
use crate::proto::StorageServiceClient;

pub struct StorageClient {
    endpoint: Endpoint,
    client: Arc<RwLock<StorageServiceClient<WrappedChannel>>>,
    disk_id: String,

    // Indexes, will be -1 until assigned a set.
    pool_index: isize,
    set_index: isize,
    disk_index: isize,
}

impl StorageClient {
    pub fn new(endpoint: Endpoint) -> anyhow::Result<Self> {
        let channel = get_inter_node_client_builder().build(&endpoint)?;
        let set_health_check = channel.health_check_setter();

        let client = Arc::new(RwLock::new(StorageServiceClient::new(channel)));

        let mut c = client.clone();
        set_health_check(Box::new(move || {
            let mut client = c.clone();
            Box::pin(async move {
                let mut client = client.write().await;
                match client.health(proto::Empty {}).await {
                    Ok(_) => true,
                    Err(_) => false,
                }
            })
        }));

        Ok(Self {
            endpoint,
            client,
            disk_id: "".to_string(),
            pool_index: 0,
            set_index: 0,
            disk_index: 0,
        })
    }
}
