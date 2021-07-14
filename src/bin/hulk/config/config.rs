use errors::AsError;
use thiserror::Error;
use tokio::io::AsyncReadExt;

use super::*;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("config file not found")]
    ConfigNotFound,
}

async fn read_config(
    api: &object::ObjectLayer,
    config_file: &str,
    data: String,
) -> anyhow::Result<Vec<u8>> {
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

async fn delete_config(api: &object::ObjectLayer, config_file: &str) -> anyhow::Result<()> {
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

async fn save_config(
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
