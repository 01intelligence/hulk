use anyhow::ensure;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::*;

mod help;
pub use help::*;

pub const EXTENSIONS: &str = "extensions";
pub const ALLOW_ENCRYPTED: &str = "allow_encryption";
pub const MIME_TYPES: &str = "mime_types";

pub const ENV_COMPRESS_STATE: &str = "HULK_COMPRESS_ENABLE";
pub const ENV_COMPRESS_ALLOW_ENCRYPTION: &str = "HULK_COMPRESS_ALLOW_ENCRYPTION";
pub const ENV_COMPRESS_EXTENSIONS: &str = "HULK_COMPRESS_EXTENSIONS";
pub const ENV_COMPRESS_MIME_TYPES: &str = "HULK_COMPRESS_MIME_TYPES";

// Include-list for compression.
pub const DEFAULT_EXTENSIONS: &str = ".txt,.log,.csv,.json,.tar,.xml,.bin";
pub const DEFAULT_MIME_TYPES: &str = "text/*,application/json,application/xml,binary/octet-stream";

lazy_static! {
    pub static ref DEFAULT_KVS: KVS = KVS(vec![
        KV {
            key: ENABLE_KEY.to_owned(),
            value: ENABLE_OFF.to_owned(),
        },
        KV {
            key: ALLOW_ENCRYPTED.to_owned(),
            value: ENABLE_OFF.to_owned(),
        },
        KV {
            key: EXTENSIONS.to_owned(),
            value: DEFAULT_EXTENSIONS.to_owned(),
        },
        KV {
            key: MIME_TYPES.to_owned(),
            value: DEFAULT_MIME_TYPES.to_owned(),
        },
    ]);
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub enabled: bool,
    pub allow_encrypted: bool,
    pub extensions: Vec<String>,
    pub mime_types: Vec<String>,
}

pub fn lookup_config(kvs: &KVS) -> anyhow::Result<Config> {
    let _ = check_valid_keys(COMPRESSION_SUB_SYS, kvs, &DEFAULT_KVS)?;

    let enabled =
        std::env::var(ENV_COMPRESS_STATE).unwrap_or_else(|_| kvs.get(ENABLE_KEY).to_owned());
    let enabled = match crate::utils::parse_bool_ext(&enabled) {
        Err(err) => {
            // Parsing failures happen due to empty KVS, ignore it.
            if kvs.is_empty() {
                return Ok(Config {
                    enabled: false,
                    ..Default::default()
                });
            }
            return Err(err);
        }
        Ok(enabled) => enabled,
    };
    if !enabled {
        return Ok(Config {
            enabled,
            ..Default::default()
        });
    }

    let allow_encrypted = std::env::var(ENV_COMPRESS_ALLOW_ENCRYPTION)
        .unwrap_or_else(|_| kvs.get(ALLOW_ENCRYPTED).to_owned());
    let allow_encrypted = crate::utils::parse_bool_ext(&allow_encrypted)?;

    let extensions =
        std::env::var(ENV_COMPRESS_EXTENSIONS).unwrap_or_else(|_| kvs.get(EXTENSIONS).to_owned());
    let extensions = parse_compress_includes(&extensions).map_err(|err| {
        anyhow::anyhow!(
            "{}: invalid HULK_COMPRESS_EXTENSIONS value '{}'",
            err,
            extensions
        )
    })?;

    let mime_types =
        std::env::var(ENV_COMPRESS_MIME_TYPES).unwrap_or_else(|_| kvs.get(MIME_TYPES).to_owned());
    let mime_types = parse_compress_includes(&mime_types).map_err(|err| {
        anyhow::anyhow!(
            "{}: invalid HULK_COMPRESS_MIME_TYPES value '{}'",
            err,
            mime_types
        )
    })?;

    Ok(Config {
        enabled,
        allow_encrypted,
        extensions,
        mime_types,
    })
}

fn parse_compress_includes(include: &str) -> anyhow::Result<Vec<String>> {
    let includes: Vec<_> = include
        .split(VALUE_SEPARATOR)
        .map(|s| s.to_owned())
        .collect();
    for e in &includes {
        ensure!(
            !e.is_empty(),
            crate::errors::UiError::InvalidCompressionIncludesValue
                .msg("extension or mime-type cannot be empty".to_owned())
        );
        ensure!(
            e != "/",
            crate::errors::UiError::InvalidCompressionIncludesValue
                .msg("extension or mime-type cannot be '/'".to_owned())
        );
    }
    Ok(includes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_compress_includes() {
        let cases: [(&str, Vec<String>, bool); 7] = [
            // Invalid input
            (",,,", Vec::new(), false),
            ("", Vec::new(), false),
            (",", Vec::new(), false),
            ("/", Vec::new(), false),
            ("text/*,/", Vec::new(), false),
            // Valid input
            (
                ".txt,.log",
                vec![".txt".to_string(), ".log".to_string()],
                true,
            ),
            (
                "text/*,application/json",
                vec!["text/*".to_string(), "application/json".to_string()],
                true,
            ),
        ];
        for (drive_str, expected_patterns, success) in cases.iter() {
            let result = parse_compress_includes(drive_str);
            match result {
                Ok(result) => assert_eq!(result, *expected_patterns),
                Err(_) => assert!(!success),
            }
        }
    }
}
