use clap::ArgMatches;
use hulk::config;
use hulk::globals::{ReadWriteGuard, Set, GLOBALS};
use hulk::utils::{PathAbsolutize, PathBuf};

use super::*;

pub async fn handle_common_cli_args(m: &ArgMatches) {
    let mut global_cli_context = GLOBALS.cli_context.write_guard();
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

pub async fn handle_common_env_vars() {
    GLOBALS.browser_enabled.set(
        utils::parse_bool_ext(
            &std::env::var(config::ENV_BROWSER).unwrap_or_else(|_| config::ENABLE_ON.to_owned()),
        )
        .expect(&format!("Invalid {} env var", config::ENV_BROWSER)),
    );

    GLOBALS.fs_osync.set(
        utils::parse_bool_ext(
            &std::env::var(config::ENV_FS_OSYNC).unwrap_or_else(|_| config::ENABLE_OFF.to_owned()),
        )
        .expect(&format!("Invalid {} env var", config::ENV_FS_OSYNC)),
    );

    let mut global_domains = GLOBALS.domain_names.write_guard();
    let domains = std::env::var(config::ENV_DOMAIN).unwrap_or_default();
    if !domains.is_empty() {
        for domain in domains.split(config::VALUE_SEPARATOR) {
            let _ = trust_dns_resolver::Name::from_utf8(domain)
                .expect(&format!("Invalid {} env var", config::ENV_DOMAIN));
            global_domains.push(domain.to_owned());
        }
        global_domains.sort_unstable();
        let suffix = lcp(&global_domains[..], false);
        for domain in &global_domains[..] {
            assert!(
                !(domain as &str == suffix && global_domains.len() > 1),
                "Overlapping domains {:?} not allowed",
                &global_domains[..]
            );
        }
    }
    drop(global_domains);

    let mut domain_ips = hulk::strset::StringSet::new();
    let public_ips = std::env::var(config::ENV_PUBLIC_IPS).unwrap_or_default();
    if !public_ips.is_empty() {
        for endpoint in public_ips.split(config::VALUE_SEPARATOR) {
            if endpoint.parse::<std::net::IpAddr>().is_err() {
                let addrs = tokio::net::lookup_host(endpoint)
                    .await
                    .expect(&format!("Invalid {} env var", config::ENV_PUBLIC_IPS));
                for addr in addrs {
                    domain_ips.add(addr.to_string());
                }
            }
            domain_ips.add(endpoint.to_owned());
        }
    } else {
        domain_ips = hulk::endpoint::get_local_ip4();
        for endpoint in GLOBALS.endpoints.guard().host_names() {
            domain_ips.add(endpoint);
        }
    }
    hulk::endpoint::update_domain_ips(&domain_ips);

    GLOBALS.inplace_update_disabled.set(
        !utils::parse_bool_ext(
            &std::env::var(config::ENV_UPDATE).unwrap_or_else(|_| config::ENABLE_OFF.to_owned()),
        )
        .expect(&format!("Invalid {} env var", config::ENV_UPDATE)),
    );

    if std::env::var(config::ENV_ROOT_PASSWORD).is_ok()
        || std::env::var(config::ENV_ROOT_USER).is_ok()
    {
        assert!(
            std::env::var(config::ENV_ROOT_USER).is_ok(),
            hulk::errors::UiError::MissingEnvCredentialRootUser
                .msg("".to_owned())
                .to_string()
        );
        assert!(
            std::env::var(config::ENV_ROOT_PASSWORD).is_ok(),
            hulk::errors::UiError::MissingEnvCredentialRootPassword
                .msg("".to_owned())
                .to_string()
        );
        let user = std::env::var(config::ENV_ROOT_USER).unwrap();
        let password = std::env::var(config::ENV_ROOT_PASSWORD).unwrap();
        *GLOBALS.active_cred.write_guard() = hulk::auth::new_credentials(user, password).expect(
            &hulk::errors::UiError::InvalidCredentials
                .msg("Unable to validate credentials from environment variables".to_owned())
                .to_string(),
        );
    }

    // TODO: KMS
    // TODO: debug remote tiers immediately
}

fn get_tls_config() -> anyhow::Result<(Vec<rustls::Certificate>, hulk::certs::Manager, bool)> {
    anyhow::bail!("");
}

// Returns the longest common suffix/prefix of the provided strings.
fn lcp<S: AsRef<str>>(strs: &[S], prefix: bool) -> &str {
    // Short circuit empty list.
    if strs.is_empty() {
        return "";
    }
    // Short circuit single-element list.
    if strs.len() == 1 {
        return strs[0].as_ref();
    }
    let mut xfix = strs[0].as_ref().as_bytes();
    // Compare first to rest.
    for s in &strs[1..] {
        let s = s.as_ref().as_bytes();
        let xfix_len = xfix.len();
        let s_len = s.len();
        // Short circuit empty strings.
        if xfix_len == 0 || s_len == 0 {
            return "";
        }
        // Maximum possible length.
        let mut max_len = xfix_len.min(s_len);
        // Compare letters.
        if prefix {
            // Prefix, iterate left to right.
            for i in 0..max_len {
                if xfix[i] != s[i] {
                    xfix = &xfix[..i];
                    break;
                }
            }
        } else {
            // Suffix, iterate right to left.
            for i in 0..max_len {
                let xi = xfix_len - i - 1;
                let si = s_len - i - 1;
                if xfix[xi] != s[si] {
                    xfix = &xfix[xi + 1..];
                    break;
                }
            }
        }
    }
    // Treat non UTF-8 bytes as empty string.
    std::str::from_utf8(xfix).unwrap_or_default()
}
