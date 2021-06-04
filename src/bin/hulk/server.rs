use std::sync::Arc;

use actix_web::{rt::System, web, App, HttpResponse, HttpServer};
use rustls::{NoClientAuth, ResolvesServerCertUsingSNI, ServerConfig};

pub struct Server<F, I, S, B> {
    pub server: HttpServer<F, I, S, B>,
}

impl<F, I, S, B> Server<F, I, S, B> {
    pub fn new() -> Self {
        let mut config = ServerConfig::new(NoClientAuth::new());
        config.set_single_cert();
        let mut resolver = Arc::new(ResolvesServerCertUsingSNI::new());
        resolver.add();
        config.cert_resolver = resolver;

        let server = HttpServer::new().bind_rustls(&[""], config);

        Server { server }
    }
}
