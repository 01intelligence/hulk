use hulk::certs;

fn get_tls_config() -> anyhow::Result<(Vec<rustls::Certificate>, certs::Manager, bool)> {}
