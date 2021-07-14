use std::collections::HashMap;

// Contains info of the underlying backend.
pub struct BackendInfo {
    // Represents various backend types, currently on FS, Erasure and Gateway
    pub type_: crate::object::BackendType,

    // Following fields are only meaningful if BackendType is Gateway.
    pub gateway_online: bool,

    // Following fields are only meaningful if BackendType is Erasure.
    pub online_disks: BackendDisks, // Online disks during server startup.
    pub offline_disks: BackendDisks, // Offline disks during server startup.

    // Following fields are only meaningful if BackendType is Erasure.
    pub standard_sc_data: Vec<usize>, // Data disks for currently configured Standard storage class.
    pub standard_sc_parity: usize, // Parity disks for currently configured Standard storage class.
    pub rr_sc_data: Vec<usize>, // Data disks for currently configured Reduced Redundancy storage class.
    pub rr_sc_parity: usize, // Parity disks for currently configured Reduced Redundancy storage class.
}

pub struct BackendDisks(HashMap<String, usize>);
