use std::sync::Arc;

use actix_web::{web, App, HttpResponse, HttpServer};
use clap::ArgMatches;
use hulk::globals::{self, Guard, ReadWriteGuard, GLOBALS};
use rustls::{NoClientAuth, ResolvesServerCertUsingSNI, ServerConfig};

use super::*;

pub async fn handle_server_cli_args(m: &ArgMatches) {
    handle_common_cli_args(m).await;

    *GLOBALS.host.guard() = GLOBALS.cli_context.guard().host.clone();
    *GLOBALS.http_port.guard() = GLOBALS.cli_context.guard().client_port.to_string();
    *GLOBALS.rpc_port.guard() = GLOBALS.cli_context.guard().peer_port.to_string();
    *GLOBALS.http_addr.guard() = hulk::endpoint::join_host_port(
        GLOBALS.host.guard().as_str(),
        GLOBALS.http_port.guard().as_str(),
    );
    *GLOBALS.rpc_addr.guard() = hulk::endpoint::join_host_port(
        GLOBALS.host.guard().as_str(),
        GLOBALS.rpc_port.guard().as_str(),
    );
}

pub async fn handle_server_env_vars() {
    handle_common_env_vars().await;
}

pub struct Server {
    pub server: actix_web::dev::Server,
}

impl Server {
    pub async fn run(m: &ArgMatches) {
        let mut event_handler = EventHandler::new();
        let event_sender = event_handler.sender();
        tokio::spawn(async move { event_handler.handle_events().await });

        bitrot::bitrot_self_test();
        erasure::erasure_self_test();
        object::compress_self_test();

        handle_server_cli_args(m).await;
        handle_server_env_vars().await;

        let mut config = ServerConfig::new(NoClientAuth::new());
        // config.set_single_cert();
        let mut resolver = Arc::new(ResolvesServerCertUsingSNI::new());
        // resolver.add();
        config.cert_resolver = resolver;

        let http_server = HttpServer::new(|| App::new())
            .bind_rustls(GLOBALS.http_addr.guard().as_str(), config)
            .unwrap();
        let http_server = http_server.disable_signals().run();
        tokio::pin!(http_server);

        let (rpc_tx, rpc_rx) = tokio::sync::oneshot::channel();
        let rpc_server = hulk::rpc::serve(
            GLOBALS.http_addr.guard().as_str().parse().unwrap(),
            GLOBALS.endpoints.guard().clone(),
            async {
                let _ = rpc_rx.await;
            },
        );
        tokio::pin!(rpc_server);

        tokio::select! {
            _ = &mut http_server => {
                let _ = rpc_tx.send(());
                let _ = rpc_server.await;
            }
            _ = &mut rpc_server => {
                http_server.stop(true).await;
                let _ = http_server.await;
            }
        }
    }
}
