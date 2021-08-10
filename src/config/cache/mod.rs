use anyhow::ensure;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::*;
use crate::errors;

mod help;
pub use help::*;

pub const DRIVES: &str = "drives";
pub const EXCLUDE: &str = "exclude";
pub const EXPIRY: &str = "expiry";
pub const MAX_USE: &str = "maxuse";
pub const QUOTA: &str = "quota";
pub const AFTER: &str = "after";
pub const WATERMARK_LOW: &str = "watermark_low";
pub const WATERMARK_HIGH: &str = "watermark_high";
pub const RANGE: &str = "range";
pub const COMMIT: &str = "commit";

pub const ENV_CACHE_DRIVES: &str = "HULK_CACHE_DRIVES";
pub const ENV_CACHE_EXCLUDE: &str = "HULK_CACHE_EXCLUDE";
pub const ENV_CACHE_EXPIRY: &str = "HULK_CACHE_EXPIRY";
pub const ENV_CACHE_MAX_USE: &str = "HULK_CACHE_MAXUSE";
pub const ENV_CACHE_QUOTA: &str = "HULK_CACHE_QUOTA";
pub const ENV_CACHE_AFTER: &str = "HULK_CACHE_AFTER";
pub const ENV_CACHE_WATERMARK_LOW: &str = "HULK_CACHE_WATERMARK_LOW";
pub const ENV_CACHE_WATERMARK_HIGH: &str = "HULK_CACHE_WATERMARK_HIGH";
pub const ENV_CACHE_RANGE: &str = "HULK_CACHE_RANGE";
pub const ENV_CACHE_COMMIT: &str = "HULK_CACHE_COMMIT";

pub const ENV_CACHE_ENCRYPTION_KEY: &str = "HULK_CACHE_ENCRYPTION_SECRET_KEY";

pub const DEFAULT_EXPIRY: &str = "90";
pub const DEFAULT_QUOTA: &str = "80";
pub const DEFAULT_AFTER: &str = "0";
pub const DEFAULT_WATER_MARK_LOW: &str = "70";
pub const DEFAULT_WATER_MARK_HIGH: &str = "80";
pub const DEFAULT_CACHE_COMMIT: &str = "writethrough";

const CACHE_DELIMITER_LEGACY: &str = ";";
const CACHE_DELIMITER: &str = ",";

lazy_static! {
    // Default storage class config
    pub static ref DEFAULT_KVS: KVS = KVS(vec![
        KV {
            key: DRIVES.to_owned(),
            value: "".to_owned(),
        },
        KV {
            key: EXCLUDE.to_owned(),
            value: "".to_owned(),
        },
        KV {
            key: EXPIRY.to_owned(),
            value: DEFAULT_EXPIRY.to_owned(),
        },
        KV {
            key: QUOTA.to_owned(),
            value: DEFAULT_QUOTA.to_owned(),
        },
        KV {
            key: AFTER.to_owned(),
            value: DEFAULT_AFTER.to_owned(),
        },
        KV {
            key: WATERMARK_LOW.to_owned(),
            value: DEFAULT_WATER_MARK_LOW.to_owned(),
        },
        KV {
            key: WATERMARK_HIGH.to_owned(),
            value: DEFAULT_WATER_MARK_HIGH.to_owned(),
        },
        KV {
            key: RANGE.to_owned(),
            value: ENABLE_ON.to_owned(),
        },
        KV {
            key: COMMIT.to_owned(),
            value: DEFAULT_CACHE_COMMIT.to_owned(),
        },
    ]);
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(skip)]
    pub enabled: bool,
    pub drives: Vec<String>,
    pub expiry: usize,
    pub max_use: usize,
    pub quota: usize,
    pub exclude: Vec<String>,
    pub after: usize,
    pub watermark_low: usize,
    pub watermark_high: usize,
    pub range: bool,
    #[serde(skip)]
    pub commit_write_back: bool,
}

pub fn lookup_config(kvs: &KVS) -> anyhow::Result<Config> {
    let _ = check_valid_keys(CACHE_SUB_SYS, kvs, &DEFAULT_KVS)?;

    let mut cfg = Config {
        enabled: true,
        ..Default::default()
    };

    let drives = std::env::var(ENV_CACHE_DRIVES).unwrap_or_else(|_| kvs.get(DRIVES).to_owned());
    cfg.drives = parse_cache_drives(&drives)?;

    let excludes = std::env::var(ENV_CACHE_EXCLUDE).unwrap_or_else(|_| kvs.get(EXCLUDE).to_owned());
    if !excludes.is_empty() {
        cfg.exclude = parse_cache_excludes(&excludes)?;
    }

    let expiry = std::env::var(ENV_CACHE_EXPIRY).unwrap_or_else(|_| kvs.get(EXPIRY).to_owned());
    if !expiry.is_empty() {
        cfg.expiry = expiry
            .parse::<usize>()
            .map_err(|e| errors::UiError::InvalidCacheExpiryValue.msg(e.to_string()))?;
    }

    let max_use = std::env::var(ENV_CACHE_MAX_USE).unwrap_or_else(|_| kvs.get(MAX_USE).to_owned());
    if !max_use.is_empty() {
        cfg.max_use = max_use
            .parse::<usize>()
            .map_err(|e| errors::UiError::InvalidCacheQuota.msg(e.to_string()))?;
        ensure!(
            cfg.max_use >= 0 && cfg.max_use <= 100,
            errors::UiError::InvalidCacheQuota
                .msg("config max use value should not be none or negative".to_owned())
        );
        cfg.quota = cfg.max_use;
    } else {
        let quota = std::env::var(ENV_CACHE_QUOTA).unwrap_or_else(|_| kvs.get(QUOTA).to_owned());
        if !quota.is_empty() {
            cfg.quota = quota
                .parse::<usize>()
                .map_err(|e| errors::UiError::InvalidCacheQuota.msg(e.to_string()))?;
            ensure!(
                cfg.quota >= 0 && cfg.quota <= 100,
                errors::UiError::InvalidCacheQuota
                    .msg("config quota value should not be none or negative".to_owned())
            );
        }
        cfg.max_use = cfg.quota;
    }

    let after = std::env::var(ENV_CACHE_AFTER).unwrap_or_else(|_| kvs.get(AFTER).to_owned());
    if !after.is_empty() {
        cfg.after = after
            .parse::<usize>()
            .map_err(|e| errors::UiError::InvalidCacheAfter.msg(e.to_string()))?;
    }

    let low_wm = std::env::var(ENV_CACHE_WATERMARK_LOW)
        .unwrap_or_else(|_| kvs.get(WATERMARK_LOW).to_owned());
    if !low_wm.is_empty() {
        cfg.watermark_low = low_wm
            .parse::<usize>()
            .map_err(|e| errors::UiError::InvalidCacheWatermarkLow.msg(e.to_string()))?;
        ensure!(
            cfg.watermark_low >= 0 && cfg.watermark_low <= 100,
            errors::UiError::InvalidCacheWatermarkLow
                .msg("config low watermark value should be between 0 and 100".to_owned())
        );
    }

    let high_wm = std::env::var(ENV_CACHE_WATERMARK_HIGH)
        .unwrap_or_else(|_| kvs.get(WATERMARK_HIGH).to_owned());
    if !high_wm.is_empty() {
        cfg.watermark_high = high_wm
            .parse::<usize>()
            .map_err(|e| errors::UiError::InvalidCacheWatermarkHigh.msg(e.to_string()))?;
        ensure!(
            cfg.watermark_high >= 0 && cfg.watermark_high <= 100,
            errors::UiError::InvalidCacheWatermarkHigh
                .msg("config high watermark value should be between 0 and 100".to_owned())
        );
    }

    ensure!(
        cfg.watermark_low <= cfg.watermark_high,
        errors::UiError::InvalidCacheWatermarkHigh.msg(
            "config high watermark value should be greater than low watermark value".to_owned()
        )
    );

    cfg.range = true;
    let range = std::env::var(ENV_CACHE_RANGE).unwrap_or_else(|_| kvs.get(RANGE).to_owned());
    if !range.is_empty() {
        cfg.range = range
            .parse::<bool>()
            .map_err(|e| errors::UiError::InvalidCacheRange.msg(e.to_string()))?;
    }

    let commit = std::env::var(ENV_CACHE_COMMIT).unwrap_or_else(|_| kvs.get(COMMIT).to_owned());
    if !commit.is_empty() {
        cfg.commit_write_back = parse_cache_commit_mode(&commit)?;
        ensure!(
            !(cfg.after > 0 && cfg.commit_write_back),
            errors::UiError::InvalidCacheSetting
                .msg("cache after cannot be used with commit writeback".to_owned())
        );
    }

    Ok(cfg)
}

fn parse_cache_drives(drives: &str) -> anyhow::Result<Vec<String>> {
    if drives.is_empty() {
        return Ok(Vec::new());
    }

    let mut drives_slice: Vec<&str> = drives.split(CACHE_DELIMITER_LEGACY).collect();
    if drives_slice.len() == 1 && drives_slice[0] == drives {
        drives_slice = drives.split(CACHE_DELIMITER).collect();
    }

    let mut endpoints = Vec::new();
    for d in drives_slice {
        ensure!(
            !d.is_empty(),
            errors::UiError::InvalidCacheDrivesValue
                .msg("cache dir cannot be an empty path".to_owned())
        );
        if crate::ellipses::has_ellipses(&[d]) {
            for p in parse_cache_drive_paths(d)? {
                endpoints.push(p);
            }
        } else {
            endpoints.push(d.to_owned());
        }
    }
    for d in &endpoints {
        ensure!(
            std::path::Path::new(d).is_absolute(),
            errors::UiError::InvalidCacheDrivesValue
                .msg(format!("cache dir should be absolute path: {}", d))
        );
    }
    Ok(endpoints)
}

fn parse_cache_drive_paths(arg: &str) -> anyhow::Result<Vec<String>> {
    match crate::ellipses::find_ellipses_patterns(arg) {
        Err(e) => {
            return Err(errors::UiError::InvalidCacheDrivesValue
                .msg(e.to_string())
                .into());
        }
        Ok(patterns) => Ok(patterns.expand().iter().map(|p| p.join("")).collect()),
    }
}

fn parse_cache_excludes(excludes: &str) -> anyhow::Result<Vec<String>> {
    if excludes.is_empty() {
        return Ok(Vec::new());
    }

    let mut excludes_slice: Vec<&str> = excludes.split(CACHE_DELIMITER_LEGACY).collect();
    if excludes_slice.len() == 1 && excludes_slice[0] == excludes {
        excludes_slice = excludes.split(CACHE_DELIMITER).collect();
    }

    for e in &excludes_slice {
        ensure!(
            !e.is_empty(),
            errors::UiError::InvalidCacheExcludesValue
                .msg(format!("cache exclude path '{}' cannot be empty", e))
        );
        ensure!(
            !e.starts_with("/"),
            errors::UiError::InvalidCacheExcludesValue.msg(format!(
                "cache exclude pattern '{}' cannot start with '/' as prefix",
                e
            ))
        );
    }
    Ok(excludes_slice.into_iter().map(|e| e.to_owned()).collect())
}

fn parse_cache_commit_mode(commit_str: &str) -> anyhow::Result<bool> {
    match &commit_str.to_lowercase() as &str {
        "wirteback" => Ok(true),
        "writethrough" => Ok(false),
        _ => Err(errors::UiError::InvalidCacheCommitValue
            .msg("cache commit value must be 'writeback' or 'writethrough'".to_owned())
            .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cache_drives() {
        let invalidCases: [(&str, Vec<String>, bool); 4] = [
            ("bucket1/*;*.png;images/trip/barcelona/*", Vec::new(), false),
            ("bucket1", Vec::new(), false),
            (";;;", Vec::new(), false),
            (",;,;,;", Vec::new(), false),
        ];
        for (driveStr, expected_patterns, success) in invalidCases.iter() {
            let result = parse_cache_drives(driveStr);
            match result {
                Ok(result) => assert_eq!(result, *expected_patterns),
                Err(_) => assert!(!success),
            }
        }
        if std::env::consts::OS == "windows" {
            let validCases: [(&str, Vec<String>, bool); 3] = [
                (
                    "C:/home/drive1;C:/home/drive2;C:/home/drive3",
                    vec![
                        "C:/home/drive1".to_string(),
                        "C:/home/drive2".to_string(),
                        "C:/home/drive3".to_string(),
                    ],
                    true,
                ),
                (
                    "C:/home/drive{1...3}",
                    vec![
                        "C:/home/drive1".to_string(),
                        "C:/home/drive2".to_string(),
                        "C:/home/drive3".to_string(),
                    ],
                    true,
                ),
                ("C:/home/drive{1..3}", Vec::new(), false),
            ];
            for (driveStr, expected_patterns, success) in validCases.iter() {
                let result = parse_cache_drives(driveStr);
                match result {
                    Ok(result) => assert_eq!(result, *expected_patterns),
                    Err(_) => assert!(!success),
                }
            }
        } else {
            let validCases: [(&str, Vec<String>, bool); 4] = [
                (
                    "/home/drive1;/home/drive2;/home/drive3",
                    vec![
                        "/home/drive1".to_string(),
                        "/home/drive2".to_string(),
                        "/home/drive3".to_string(),
                    ],
                    true,
                ),
                (
                    "/home/drive1,/home/drive2,/home/drive3",
                    vec![
                        "/home/drive1".to_string(),
                        "/home/drive2".to_string(),
                        "/home/drive3".to_string(),
                    ],
                    true,
                ),
                (
                    "/home/drive{1...3}",
                    vec![
                        "/home/drive1".to_string(),
                        "/home/drive2".to_string(),
                        "/home/drive3".to_string(),
                    ],
                    true,
                ),
                ("/home/drive{1..3}", Vec::new(), false),
            ];
            for (driveStr, expected_patterns, success) in validCases.iter() {
                let result = parse_cache_drives(driveStr);
                match result {
                    Ok(result) => assert_eq!(result, *expected_patterns),
                    Err(_) => assert!(!success),
                }
            }
        }
    }

    #[test]
    fn test_parse_cache_exclude() {
        let cases: [(&str, Vec<String>, bool); 6] = [
            // Invalid input
            ("/home/drive1;/home/drive2;/home/drive3", Vec::new(), false),
            ("/", Vec::new(), false),
            (";;;", Vec::new(), false),
            // Valid input
            (
                "bucket1/*;*.png;images/trip/barcelona/*",
                vec![
                    "bucket1/*".to_string(),
                    "*.png".to_string(),
                    "images/trip/barcelona/*".to_string(),
                ],
                true,
            ),
            (
                "bucket1/*,*.png,images/trip/barcelona/*",
                vec![
                    "bucket1/*".to_string(),
                    "*.png".to_string(),
                    "images/trip/barcelona/*".to_string(),
                ],
                true,
            ),
            ("bucket1", vec!["bucket1".to_string()], true),
        ];
        for (driveStr, expected_patterns, success) in cases.iter() {
            let result = parse_cache_excludes(driveStr);
            match result {
                Ok(result) => assert_eq!(result, *expected_patterns),
                Err(_) => assert!(!success),
            }
        }
    }
}
