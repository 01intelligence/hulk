use clap::ArgMatches;

use super::*;

pub fn handle_common_cli_args(m: &ArgMatches) {
    let mut global_cli_context = GLOBALS.cli_context.guard();
    global_cli_context.json = m
        .value_of("json")
        .map_or(false, |v| v.parse::<bool>().unwrap());
    global_cli_context.quiet = m
        .value_of("quiet")
        .map_or(false, |v| v.parse::<bool>().unwrap());
    global_cli_context.anonymous = m
        .value_of("anonymous")
        .map_or(false, |v| v.parse::<bool>().unwrap());
    global_cli_context.host = m
        .value_of("host")
        .unwrap_or(globals::GLOBAL_DEFAULT_HOST)
        .to_owned();
    global_cli_context.http_port = m
        .value_of_t("http-port")
        .unwrap_or(globals::GLOBAL_DEFAULT_HTTP_PORT);
    global_cli_context.rpc_port = m
        .value_of_t("rpc-port")
        .unwrap_or(globals::GLOBAL_DEFAULT_RPC_PORT);
    global_cli_context.strict_s3_compatibility = m
        .value_of("no-s3-compatibility")
        .map_or(false, |v| v.parse::<bool>().unwrap());
}

fn get_tls_config() -> anyhow::Result<(Vec<rustls::Certificate>, certs::Manager, bool)> {
    anyhow::bail!("");
}
