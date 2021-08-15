use std::collections::HashMap;
use std::io::BufReader;
use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use log::error;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use rustls::internal::pemfile::{certs as extract_certs, pkcs8_private_keys};
use rustls::sign::{any_supported_type, CertifiedKey};
use rustls::{ClientHello, ResolvesServerCert};
use tokio::io::AsyncReadExt;

use crate::fs::File;
use crate::utils::Path;

pub struct Manager {
    certs: Arc<RwLock<HashMap<KeyCert, CertifiedKey>>>,
    default_cert: KeyCert,
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct KeyCert {
    key_file: String,
    cert_file: String,
}

impl Manager {
    pub fn new() -> Self {
        todo!()
    }

    async fn watch_file_events(&mut self) -> anyhow::Result<()> {
        let certs = Arc::clone(&self.certs);
        let handler = move |res: notify::Result<notify::Event>| -> anyhow::Result<()> {
            match res {
                Ok(event) => match event.kind {
                    notify::EventKind::Create(notify::event::CreateKind::File)
                    | notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                        let path = event
                            .paths
                            .last()
                            .ok_or_else(|| anyhow!("empty path"))?
                            .as_path();
                        let mut certs = certs.write().unwrap();
                        let mut new_certs = HashMap::new();
                        for pair in certs.keys() {
                            if path != Path::new(&pair.key_file)
                                && path != Path::new(&pair.cert_file)
                            {
                                continue;
                            }
                            let (cert_file, key_file) =
                                tokio::task::block_in_place(|| -> std::io::Result<_> {
                                    tokio::runtime::Handle::current().block_on(async {
                                        let mut cert_file = Vec::new();
                                        File::open(&pair.cert_file)
                                            .await?
                                            .read_to_end(&mut cert_file)
                                            .await?;
                                        let mut key_file = Vec::new();
                                        File::open(&pair.key_file)
                                            .await?
                                            .read_to_end(&mut key_file)
                                            .await?;
                                        Ok((cert_file, key_file))
                                    })
                                })?;

                            let cert_file = &mut BufReader::new(&cert_file[..]);
                            let key_file = &mut BufReader::new(&key_file[..]);
                            let cert_chain =
                                extract_certs(cert_file).map_err(|_| anyhow!("invalid certs"))?;
                            cert_chain.first().ok_or_else(|| anyhow!("invalid certs"))?;
                            let mut keys = pkcs8_private_keys(key_file)
                                .map_err(|_| anyhow!("invalid private key"))?;
                            let key = any_supported_type(
                                keys.first().ok_or_else(|| anyhow!("invalid private key"))?,
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
        watcher.unwatch(".");
        Ok(())
    }
}

struct ResolvesServerCertUsingSNI {
    manager: &'static Manager,
    resolver: rustls::ResolvesServerCertUsingSNI,
}

impl ResolvesServerCert for ResolvesServerCertUsingSNI {
    fn resolve(&self, client_hello: ClientHello) -> Option<CertifiedKey> {
        if client_hello.server_name().is_none() {
            let certs = self.manager.certs.read().unwrap();
            Some(certs.get(&self.manager.default_cert).unwrap().clone())
        } else {
            self.resolver.resolve(client_hello)
        }
    }
}
