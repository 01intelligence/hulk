use lazy_static::lazy_static;

lazy_static! {
    pub static ref is_kubernetes: bool = std::env::var("KUBERNETES_SERVICE_HOST").is_ok();
    pub static ref is_kubernetes_replicaset: bool =
        *is_kubernetes && std::env::var("KUBERNETES_REPLICA_SET").is_ok();
}
