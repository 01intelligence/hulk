// Reduced redundancy storage class
pub const RRS: &str = "REDUCED_REDUNDANCY";
// Standard storage class
pub const STANDARD: &str = "STANDARD";
// DMA storage class
pub const DMA: &str = "DMA";

// Valid values are "write" and "read+write"
pub const DMA_WRITE: &str = "write";
pub const DMA_READ_WRITE: &str = "read+write";

pub const CLASS_STANDARD: &str = "standard";
pub const CLASS_RRS: &str = "rrs";
pub const CLASS_DMA: &str = "dma";

// Reduced redundancy storage class environment variable
pub const RRS_ENV: &str = "MINIO_STORAGE_CLASS_RRS";
// Standard storage class environment variable
pub const STANDARD_ENV: &str = "MINIO_STORAGE_CLASS_STANDARD";
// DMA storage class environment variable
pub const DMA_ENV: &str = "MINIO_STORAGE_CLASS_DMA";

// Supported storage class scheme is EC
const SCHEME_PREFIX: &str = "EC";

// Min parity disks
const MIN_PARITY_DISKS: usize = 2;

// Default RRS parity is always minimum parity.
const DEFAULT_RRS_PARITY: usize = MIN_PARITY_DISKS;

// Default DMA value
const DEFAULT_DMA: &str = DMA_READ_WRITE;
