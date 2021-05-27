use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use anyhow::anyhow;
use log::error;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use rustls::internal::pemfile::{certs as extract_certs, pkcs8_private_keys};
use rustls::sign::{any_supported_type, CertifiedKey};

struct Manager {
    certs: Arc<std::sync::RwLock<HashMap<KeyCert, CertifiedKey>>>,
    default_cert: KeyCert,
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct KeyCert {
    key_file: String,
    cert_file: String,
}

impl Manager {
    fn watch_file_events(&mut self) -> anyhow::Result<()> {
        let certs = Arc::clone(&self.certs);
        let handler = move |res: notify::Result<notify::Event>| -> anyhow::Result<()> {
            match res {
                Ok(event) => match event.kind {
                    notify::EventKind::Create(notify::event::CreateKind::File)
                    | notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                        let path = event.paths.last().ok_or(anyhow!("empty path"))?.as_path();
                        let mut certs = certs.write().unwrap();
                        let mut new_certs = HashMap::new();
                        for (pair, _) in &*certs {
                            if path != Path::new(&pair.key_file)
                                && path != Path::new(&pair.cert_file)
                            {
                                continue;
                            }
                            let cert_file = &mut BufReader::new(File::open(&pair.cert_file)?);
                            let key_file = &mut BufReader::new(File::open(&pair.key_file)?);
                            let cert_chain =
                                extract_certs(cert_file).map_err(|_| anyhow!("invalid certs"))?;
                            cert_chain.first().ok_or(anyhow!("invalid certs"))?;
                            let mut keys = pkcs8_private_keys(key_file)
                                .map_err(|_| anyhow!("invalid private key"))?;
                            let key = any_supported_type(
                                keys.first().ok_or(anyhow!("invalid private key"))?,
                            )
                            .map_err(|_| anyhow!("invalid private key"))?;
                            new_certs
                                .insert(pair.clone(), CertifiedKey::new(cert_chain, Arc::new(key)));
                        }
                        certs.extend(new_certs.into_iter());
                        Ok(())
                    }
                    _ => Ok(()),
                },
                Err(err) => Err(anyhow!(err)),
            }
        };

        let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| {
            if let Err(err) = handler(res) {
                error!("watch file events error: {:?}", err);
            }
        })?;
        let conf_res = watcher.configure(Config::PreciseEvents(true))?;
        if !conf_res {
            anyhow::bail!("watcher does not support or implement the PreciseEvents config");
        }
        watcher.watch(".", RecursiveMode::Recursive)?;
        Ok(())
    }
}
