use std::sync::Arc;

use actix_web::rt::System;
use actix_web::{web, App, HttpResponse, HttpServer};
use rustls::{NoClientAuth, ResolvesServerCertUsingSNI, ServerConfig};

pub struct Server {
    pub server: actix_web::dev::Server,
}

impl Server {
    pub async fn run() {
        let mut config = ServerConfig::new(NoClientAuth::new());
        // config.set_single_cert();
        let mut resolver = Arc::new(ResolvesServerCertUsingSNI::new());
        // resolver.add();
        config.cert_resolver = resolver;

        let http_server = HttpServer::new(|| App::new())
            .bind_rustls("", config)
            .unwrap();
        let server = http_server.run();
        let _ = server.await;
    }
}
