use clap::ArgMatches;
use hulk::globals::{Guard, GLOBALS};
use hulk::utils::{PathAbsolutize, PathBuf};

use super::*;

pub async fn handle_common_cli_args(m: &ArgMatches) {
    let mut global_cli_context = GLOBALS.cli_context.guard();
    global_cli_context.quiet = m
        .value_of("quiet")
        .map_or(false, |v| v.parse::<bool>().unwrap());
    global_cli_context.anonymous = m
        .value_of("anonymous")
        .map_or(false, |v| v.parse::<bool>().unwrap());
    global_cli_context.json = m
        .value_of("json")
        .map_or(false, |v| v.parse::<bool>().unwrap());
    global_cli_context.strict_s3_compatibility = m
        .value_of("no-s3-compatibility")
        .map_or(false, |v| v.parse::<bool>().unwrap());

    global_cli_context.host = m.value_of_t("host").unwrap();
    global_cli_context.client_port = m.value_of_t("client-port").unwrap();
    global_cli_context.peer_port = m.value_of_t("peer-port").unwrap();

    let global_certs_dir: PathBuf = m.value_of_t("certs-dir").unwrap();
    let global_certs_dir = global_certs_dir.absolutize().unwrap().into_owned();
    hulk::fs::mkdir_all(&global_certs_dir, 0700).await.unwrap();
    *GLOBAL_CERTS_DIR.write().unwrap() = global_certs_dir;
    hulk::fs::mkdir_all(&get_certs_ca_dir(), 0700)
        .await
        .unwrap();
}

fn get_tls_config() -> anyhow::Result<(Vec<rustls::Certificate>, hulk::certs::Manager, bool)> {
    anyhow::bail!("");
}
