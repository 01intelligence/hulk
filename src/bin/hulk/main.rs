use clap::{crate_authors, App, Arg};
use common::*;
use config::*;
use event::*;
use hulk::globals::*;
use hulk::router::middlewares::*;
use hulk::*;
use lazy_static::lazy_static;
use server::*;

// mod service;
mod common;
mod config;
mod event;
mod server;

lazy_static! {
    static ref version_info: String = {
        let build_time = option_env!("HULK_BUILD_TIME");
        hulk::version::hulk_version_info(build_time)
    };
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    trait AppExt {
        fn ext(self) -> Self;
    }

    impl<'a> AppExt for App<'a> {
        fn ext(self) -> Self {
            self.author(crate_authors!())
                .version(version_info.as_str())
                .long_version(version_info.as_str())
                .help_template(
                    "\
                    {before-help}{bin} - {about}\n\
                    {version}\n\n\
                    {usage-heading}\n    {usage}\n\
                    \n\
                    {all-args}{after-help}\
                    ",
                )
        }
    }

    let matches = App::new("Hulk")
        .about("A high performance object storage powered by Rust and Raft")
        .ext()
        .arg(
            Arg::new("certs-dir")
                .short('s')
                .long("certs-dir")
                .about("Sets the certs directory"),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .about("Disable startup information"),
        )
        .arg(
            Arg::new("anonymous")
                .short('a')
                .long("anonymous")
                .about("Hide sensitive information from logging"),
        )
        .arg(
            Arg::new("json")
                .short('j')
                .long("json")
                .validator(is_bool)
                .about("Output server logs and startup information in json format"),
        )
        .arg(
            Arg::new("no-s3-compatibility")
                .long("no-s3-compatibility")
                .about("Disable strict S3 compatibility by turning on certain performance optimizations")
                .hidden(true),
        )
        .subcommand(App::new("server")
            .about("Run object storage server")
            .ext()
        )
        .subcommand(App::new("gateway")
            .about("Run object storage gateway")
            .ext()
        )
        .get_matches();

    match matches.subcommand() {
        Some(("server", sub_m)) => {
            Server::run(sub_m).await;
            Ok(())
        }
        Some(("gateway", sub_m)) => Ok(()),
        _ => Ok(()),
    }
}

fn is_bool(v: &str) -> Result<(), String> {
    utils::parse_bool(v).map(|_| ()).map_err(|e| e.to_string())
}
