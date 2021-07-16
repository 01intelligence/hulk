mod help;

use anyhow::{anyhow, bail, ensure};
pub use help::*;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::config::{KV, KVS};
use crate::config::{check_valid_keys, STORAGE_CLASS_SUB_SYS};

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
pub const RRS_ENV: &str = "HULK_STORAGE_CLASS_RRS";
// Standard storage class environment variable
pub const STANDARD_ENV: &str = "HULK_STORAGE_CLASS_STANDARD";
// DMA storage class environment variable
pub const DMA_ENV: &str = "HULK_STORAGE_CLASS_DMA";

// Supported storage class scheme is EC
const SCHEME_PREFIX: &str = "EC";

// Min parity disks
const MIN_PARITY_DISKS: u8 = 2;

// Default RRS parity is always minimum parity.
const DEFAULT_RRS_PARITY: u8 = MIN_PARITY_DISKS;

// Default DMA value
const DEFAULT_DMA: &str = DMA_READ_WRITE;

lazy_static! {
    // Default storage class config
    pub static ref DEFAULT_KVS: KVS = KVS(vec![
        KV {
            key: CLASS_STANDARD.to_owned(),
            value: "".to_owned(),
        },
        KV {
            key: CLASS_RRS.to_owned(),
            value: "EC:2".to_owned(),
        },
        KV {
            key: CLASS_DMA.to_owned(),
            value: DEFAULT_DMA.to_owned(),
        },
    ]);
}

#[derive(Serialize, Deserialize, Default)]
pub struct StorageClass {
    pub parity: u8,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub standard: StorageClass,
    pub rrs: StorageClass,
    pub dma: String,
}

pub fn is_valid(sc: &str) -> bool {
    sc == RRS || sc == STANDARD
}

impl ToString for StorageClass {
    fn to_string(&self) -> String {
        if self.parity != 0 {
            format!("{}:{}", SCHEME_PREFIX, self.parity)
        } else {
            "".to_owned()
        }
    }
}

impl Config {
    // Returns the data and parity drive count based on storage class
    // If storage class is set using the env vars HULK_STORAGE_CLASS_RRS and
    // HULK_STORAGE_CLASS_STANDARD or server config fields corresponding values are
    // returned.
    //
    // -- if input storage class is empty then standard is assumed
    // -- if input is RRS but RRS is not configured default '2' parity
    //    for RRS is assumed
    // -- if input is STANDARD but STANDARD is not configured '0' parity
    //    is returned, the caller is expected to choose the right parity
    //    at that point.
    pub fn get_parity_for_sc(&self, sc: &str) -> u8 {
        if sc.trim() == RRS {
            // Set the RRS parity if available
            if self.rrs.parity == 0 {
                DEFAULT_RRS_PARITY
            } else {
                self.rrs.parity
            }
        } else {
            self.standard.parity
        }
    }
}

// Parses given env string and returns a storageClass structure.
// Supported Storage Class format is "Scheme:Number of parity disks".
// Currently only supported scheme is "EC".
fn parse_storage_class(sc: &str) -> anyhow::Result<StorageClass> {
    let s: Vec<&str> = sc.split(':').collect();
    if s.len() > 2 {
        bail!("too many sections in '{}'", sc);
    } else if s.len() < 2 {
        bail!("too few sections in '{}'", sc);
    }

    // Only allowed scheme is "EC".
    if s[0] != SCHEME_PREFIX {
        bail!("supported scheme is 'EC', but not '{}'", s[0]);
    }

    let parity = s[1]
        .parse::<u8>()
        .map_err(|_| anyhow!("invalid parity '{}'", s[1]))?;

    Ok(StorageClass { parity })
}

// Validates the parity disks.
pub fn validate_parity(ss_parity: u8, rrs_parity: u8, set_drive_count: u8) -> anyhow::Result<()> {
    // SS parity disks should be greater than or equal to minParityDisks.
    // Parity below minParityDisks is not supported.
    if ss_parity > 0 && ss_parity < MIN_PARITY_DISKS {
        bail!(
            "Standard storage class parity {} should be greater than or equal to {}",
            ss_parity,
            MIN_PARITY_DISKS
        );
    }
    // RRS parity disks should be greater than or equal to minParityDisks.
    // Parity below minParityDisks is not supported.
    if rrs_parity > 0 && rrs_parity < MIN_PARITY_DISKS {
        bail!(
            "Reduced redundancy storage class parity {} should be greater than or equal to {}",
            rrs_parity,
            MIN_PARITY_DISKS
        );
    }
    if ss_parity > set_drive_count / 2 {
        bail!(
            "Standard storage class parity {} should be less than or equal to {}",
            ss_parity,
            set_drive_count / 2
        );
    }
    if rrs_parity > set_drive_count / 2 {
        bail!(
            "Reduced redundancy storage class parity {} should be less than or equal to {}",
            rrs_parity,
            set_drive_count
        );
    }
    if ss_parity > 0 && rrs_parity > 0 && ss_parity < rrs_parity {
        bail!("Standard storage class parity disks {} should be greater than or equal to Reduced redundancy storage class parity disks {}", ss_parity, rrs_parity);
    }
    Ok(())
}

pub fn lookup_config(kvs: &KVS, set_drive_count: u8) -> anyhow::Result<Config> {
    let _ = check_valid_keys(STORAGE_CLASS_SUB_SYS, kvs, &DEFAULT_KVS)?;
    let standard =
        std::env::var(STANDARD_ENV).unwrap_or_else(|_| kvs.get(CLASS_STANDARD).to_owned());
    let rrs = std::env::var(RRS_ENV).unwrap_or_else(|_| kvs.get(CLASS_RRS).to_owned());
    let dma = std::env::var(DMA_ENV).unwrap_or_else(|_| kvs.get(CLASS_DMA).to_owned());
    let mut cfg = Config::default();
    cfg.standard = parse_storage_class(&standard)?;
    cfg.rrs = parse_storage_class(&rrs)?;
    if cfg.rrs.parity == 0 {
        cfg.rrs.parity = DEFAULT_RRS_PARITY;
    }
    cfg.dma = if dma.is_empty() {
        DEFAULT_DMA.to_owned()
    } else {
        dma
    };
    ensure!(
        cfg.dma == DMA_READ_WRITE || cfg.dma == DMA_WRITE,
        "valid dma values are 'read+write' and 'write'"
    );
    let _ = validate_parity(cfg.standard.parity, cfg.rrs.parity, set_drive_count)?;
    Ok(cfg)
}
