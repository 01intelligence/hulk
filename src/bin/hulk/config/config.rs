use super::*;

async fn read_config(api: &object::ObjectLayer, config_file: &str, data: String) -> anyhow::Result<()> {
    let r = api.get_object_and_info(object::SYSTEM_META_BUCKET, config_file, Default::default(), &Default::default(), object::LockType::Read, None).await?;
    Ok(())
}
