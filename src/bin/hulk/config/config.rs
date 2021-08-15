use bstr::ByteSlice;
use const_format::concatcp;
use hulk::admin::ConfigHistoryEntry;
use hulk::errors::AsError;
use hulk::object::{ObjectLayer, SYSTEM_META_BUCKET};
use hulk::utils::StrExt;
use hulk::{admin, config, hash, object};
use thiserror::Error;
use tokio::io::AsyncReadExt;

const SYSTEM_CONFIG_PREFIX: &str = "config";
const KV_PREFIX: &str = ".kv";
const SYSTEM_CONFIG_HISTORY_PREFIX: &str = concatcp!(SYSTEM_CONFIG_PREFIX, "/history");
const SYSTEM_CONFIG_FILE: &str = "config.json";

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("config file not found")]
    ConfigNotFound,
}

pub fn is_config_not_found(err: &anyhow::Error) -> bool {
    if let Some(&ConfigError::ConfigNotFound) = err.as_error::<ConfigError>() {
        true
    } else {
        false
    }
}

pub async fn check_config(api: &object::ObjectLayer, config_file: &str) -> anyhow::Result<()> {
    match api
        .get_object_info(object::SYSTEM_META_BUCKET, config_file, None)
        .await
    {
        Err(err) => {
            if object::is_object_not_found(&err) {
                return Err(ConfigError::ConfigNotFound.into());
            }
            return Err(err);
        }
        Ok(_) => Ok(()),
    }
}

pub async fn read_config(api: &object::ObjectLayer, config_file: &str) -> anyhow::Result<Vec<u8>> {
    match api
        .get_object_and_info(
            object::SYSTEM_META_BUCKET,
            config_file,
            Default::default(),
            &Default::default(),
            object::LockType::Read,
            None,
        )
        .await
    {
        Err(err) => {
            // Treat object not found as config not found.
            if object::is_object_not_found(&err) {
                return Err(ConfigError::ConfigNotFound.into());
            }
            return Err(err);
        }
        Ok(mut r) => {
            let mut buf = Vec::new();
            let _ = r.reader.read_to_end(&mut buf).await?;
            if buf.is_empty() {
                return Err(ConfigError::ConfigNotFound.into());
            }
            return Ok(buf);
        }
    }
}

pub async fn save_config(
    api: &object::ObjectLayer,
    config_file: &str,
    data: &[u8],
) -> anyhow::Result<()> {
    let hash_reader = hash::Reader::new(
        data,
        data.len() as isize,
        "",
        &hash::sha256_hex(data),
        data.len(),
    )?;
    let _ = api
        .put_object(
            object::SYSTEM_META_BUCKET,
            config_file,
            &mut object::PutObjectReader {},
            Some(object::ObjectOptions {
                max_parity: true,
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}

pub async fn delete_config(api: &object::ObjectLayer, config_file: &str) -> anyhow::Result<()> {
    match api
        .delete_object(object::SYSTEM_META_BUCKET, config_file, None)
        .await
    {
        Err(err) => {
            if object::is_object_not_found(&err) {
                return Err(ConfigError::ConfigNotFound.into());
            }
            return Err(err);
        }
        Ok(_) => Ok(()),
    }
}

pub async fn check_server_config(api: &object::ObjectLayer) -> anyhow::Result<()> {
    let config_file = object::path_join(&[SYSTEM_CONFIG_PREFIX, SYSTEM_CONFIG_FILE]);
    check_config(api, &config_file).await
}

pub async fn read_server_config(api: &object::ObjectLayer) -> anyhow::Result<config::Config> {
    let config_file = object::path_join(&[SYSTEM_CONFIG_PREFIX, SYSTEM_CONFIG_FILE]);
    match read_config(api, &config_file).await {
        Err(err) => {
            if object::is_object_not_found(&err) && object::get_object_layer().is_none() {
                return Ok(config::Config::new());
            }
            Err(err)
        }
        Ok(data) => {
            // TODO: KMS
            let cfg: config::Config = serde_json::from_str(data.to_str()?)?;
            // Add any missing entries
            Ok(cfg.merge_default())
        }
    }
}

pub async fn save_server_config(api: &ObjectLayer, cfg: &config::Config) -> anyhow::Result<()> {
    let data = serde_json::to_string(cfg)?;
    let config_file = object::path_join(&[SYSTEM_CONFIG_PREFIX, SYSTEM_CONFIG_FILE]);
    // TODO: KMS
    save_config(api, &config_file, data.as_bytes()).await
}

pub async fn read_server_config_history(
    api: &ObjectLayer,
    uuid_kv: &str,
) -> anyhow::Result<Vec<u8>> {
    let history_file = object::path_join(&[
        SYSTEM_CONFIG_HISTORY_PREFIX,
        &(uuid_kv.to_string() + KV_PREFIX),
    ]);
    let data = read_config(api, &history_file).await?;
    // TODO: KMS
    Ok(data)
}

pub async fn list_server_config_history(
    api: &ObjectLayer,
    with_data: bool,
    mut count: usize,
) -> anyhow::Result<Vec<ConfigHistoryEntry>> {
    let mut config_history = Vec::new();
    let mut marker = "".to_string();
    loop {
        let res = api
            .list_objects(
                SYSTEM_META_BUCKET,
                SYSTEM_CONFIG_HISTORY_PREFIX,
                &marker,
                "",
                object::MAX_OBJECT_LIST,
            )
            .await?;
        for obj in &res.objects {
            let obj_name = hulk::utils::Path::new(&obj.name)
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("invalid config history entry restore_id"))?;
            let data = if with_data {
                let data = read_config(api, &obj.name).await?;
                // TODO: KMS
                String::from_utf8(data)?
            } else {
                "".to_string()
            };
            let cfg_entry = admin::ConfigHistoryEntry {
                restore_id: obj_name.may_strip_suffix(KV_PREFIX).to_string(),
                create_time: obj.mod_time, // mod_time is create_time for config history entries
                data,
            };
            config_history.push(cfg_entry);
            count -= 1;
            if count <= 0 {
                break;
            }
        }
        if !res.is_truncated {
            break;
        }
        marker = res.next_marker.unwrap();
    }
    config_history.sort_unstable_by_key(|c| c.create_time);
    Ok(config_history)
}

pub async fn save_server_config_history(api: &ObjectLayer, kv: &[u8]) -> anyhow::Result<()> {
    let uuid_kv = uuid::Uuid::new_v4().to_string() + KV_PREFIX;
    let history_file = object::path_join(&[SYSTEM_CONFIG_HISTORY_PREFIX, &uuid_kv]);
    // TODO: KMS
    save_config(api, &history_file, kv).await
}

pub async fn delete_server_config_history(api: &ObjectLayer, uuid_kv: &str) -> anyhow::Result<()> {
    let uuid_kv = uuid_kv.to_string() + KV_PREFIX;
    let history_file = object::path_join(&[SYSTEM_CONFIG_HISTORY_PREFIX, &uuid_kv]);
    delete_config(api, &history_file).await
}
