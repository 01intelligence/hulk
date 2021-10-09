use std::cell::RefCell;
use std::fmt::Arguments;

use slog::{Drain, Key, OwnedKVList, Record, SerdeValue, KV};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use typetag::erased_serde;

use crate::utils;

pub struct WebhookDrain {
    tx: Sender<Vec<u8>>,
    name: String,
    endpoint: String,
}

impl WebhookDrain {
    pub fn new(
        name: String,
        endpoint: String,
        user_agent: String,
        auth_token: Option<String>,
    ) -> Self {
        let (tx, mut rx) = channel(10000);
        let client = reqwest::Client::builder().build().unwrap(); // TODO
        let drain = WebhookDrain {
            tx,
            name,
            endpoint: endpoint.clone(),
        };

        tokio::spawn(async move {
            use http::{header, HeaderValue, StatusCode};
            while let Some(json) = rx.recv().await {
                let mut req = client
                    .post(&endpoint)
                    .timeout(utils::seconds(5))
                    .header(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    )
                    .header(
                        header::USER_AGENT,
                        HeaderValue::from_str(&user_agent).unwrap(),
                    )
                    .body(json);
                if let Some(auth_token) = &auth_token {
                    req = req.header(
                        header::AUTHORIZATION,
                        HeaderValue::from_str(auth_token).unwrap(),
                    );
                }
                match req.send().await {
                    Ok(rep) => match rep.status() {
                        StatusCode::OK => {}
                        StatusCode::FORBIDDEN => {
                            println!("{} returned '{}', please check if your auth token is correctly set", endpoint, rep.status());
                        }
                        status => {
                            println!(
                                "{} returned '{}', please check your endpoint configuration",
                                endpoint, status,
                            );
                        }
                    },
                    Err(err) => {
                        println!(
                            "{} returned '{}', please check your endpoint configuration",
                            endpoint, err,
                        );
                    }
                }
            }
        });

        drain
    }

    pub fn target(&self) -> super::Target {
        super::Target {
            name: self.name.clone(),
            endpoint: Some(self.endpoint.clone()),
        }
    }
}

impl Drain for WebhookDrain {
    type Ok = ();
    type Err = std::io::Error;

    fn log(&self, record: &Record, values: &OwnedKVList) -> std::io::Result<()> {
        let mut buf = Vec::with_capacity(128);
        {
            let mut ser = serde_json::Serializer::pretty(&mut buf);
            let mut serializer = WebhookSerializer {
                ser: Box::new(<dyn erased_serde::Serializer>::erase(&mut ser)),
            };

            // values.serialize(record, &mut serializer)?;
            record.kv().serialize(record, &mut serializer)?;
        }

        tokio::task::block_in_place(move || {
            let _ = self.tx.blocking_send(buf);
        });
        Ok(())
    }
}

struct WebhookSerializer<'a> {
    ser: Box<dyn erased_serde::Serializer + 'a>,
}

impl<'a> slog::Serializer for WebhookSerializer<'a> {
    fn emit_arguments(&mut self, key: Key, val: &Arguments) -> slog::Result {
        // Deny any value, excluding `SerdeValue`.
        Err(slog::Error::Other)
    }

    fn emit_serde(&mut self, key: Key, value: &SerdeValue) -> slog::Result {
        value
            .as_serde()
            .erased_serialize(&mut self.ser)
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("serde serialization error: {}", e),
                )
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn test_logger_webhook() {
        use actix_web::{web, App, HttpRequest, HttpServer, Responder};

        async fn index(req: HttpRequest, json: String) -> impl Responder {
            println!("Logging json: '{}'", json);
            let tx = req.app_data::<Sender<()>>().unwrap();
            tx.send(()).await;
            json
        }

        let (tx, mut rx) = channel::<()>(1);

        actix_rt::spawn(async move {
            HttpServer::new(move || {
                App::new()
                    .app_data(tx.clone())
                    .route("/", web::post().to(index))
            })
            .bind(("127.0.0.1", 8080))
            .unwrap()
            .run()
            .await;
        });

        let drain = WebhookDrain::new(
            "TestWebhookDrain".to_owned(),
            "http://127.0.0.1:8080".to_owned(),
            "".to_owned(),
            None,
        )
        .fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let log = slog::Logger::root(drain, slog::slog_o!());

        use super::super::log;
        let entry = log::Entry {
            deployment_id: "deployment_id".to_string(),
            level: "level".to_string(),
            kind: super::super::ErrKind::System,
            time: "time".to_string(),
            api: log::Api {
                name: "name".to_string(),
                args: Some(log::Args {
                    bucket: "bucket".to_string(),
                    object: "object".to_string(),
                    metadata: Default::default(),
                }),
            },
            remote_host: "remote_host".to_string(),
            host: "host".to_string(),
            request_id: "request_id".to_string(),
            user_agent: "user_agent".to_string(),
            message: "message".to_string(),
            trace: log::Trace {
                message: "".to_string(),
                source: vec![],
                variables: Default::default(),
            },
        };

        slog::info!(log, ""; entry.clone());
        rx.recv().await;

        slog::info!(log, ""; entry);
        rx.recv().await;
    }
}
