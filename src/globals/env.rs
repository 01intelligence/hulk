use lazy_static::lazy_static;

lazy_static! {
    pub static ref IS_KUBERNETES: bool = std::env::var("KUBERNETES_SERVICE_HOST").is_ok();
    pub static ref IS_KUBERNETES_REPLICASET: bool =
        *IS_KUBERNETES && std::env::var("KUBERNETES_REPLICA_SET").is_ok();
}
