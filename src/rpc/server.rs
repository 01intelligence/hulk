use std::future::Future;
use std::net::SocketAddr;

use super::*;
use crate::proto;

pub async fn serve<F: Future<Output = ()>>(
    addr: SocketAddr,
    endpoints: crate::endpoint::EndpointServerPools,
    shutdown: F,
) -> anyhow::Result<()> {
    let _ = tonic::transport::Server::builder()
        .add_service(proto::StorageServiceServer::new(StorageService::new(endpoints).await))
        .serve_with_shutdown(addr, shutdown)
        .await?;
    Ok(())
}
