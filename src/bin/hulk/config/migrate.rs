use hulk::info;

fn log_config_migrate_msg(config_file: &str, src_version: &str, dst_version: &str) {
    info!(
        "Configuration file {} migrated from version '{}' to '{}' successfully",
        config_file, src_version, dst_version
    );
}
