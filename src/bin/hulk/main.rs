// mod service;
// mod server;
mod common;
mod globals;
mod setup_type;

use clap::{crate_authors, App, Arg};
use common::*;
use globals::*;
use setup_type::*;

fn main() {
    let build_time = option_env!("HULK_BUILD_TIME");
    let version_info = hulk::version::hulk_version_info(build_time);

    let matches = App::new("Hulk")
        .about("A high performance object storage powered by Rust and Raft")
        .author(crate_authors!())
        .version(version_info.as_ref())
        .long_version(version_info.as_ref())
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
            Arg::new("no-s3-compatibility")
                .long("no-s3-compatibility")
                .about("Disable strict S3 compatibility by turning on certain performance optimizations")
                .hidden(true),
        )
        .help_template(
            "\
            {before-help}{bin} - {about}\n\
            {version}\n\n\
            {usage-heading}\n    {usage}\n\
            \n\
            {all-args}{after-help}\
        ",
        )
        .subcommand(App::new("server").about("Run object storage server"))
        .subcommand(App::new("gateway").about("Run object storage gateway"))
        .get_matches();

    match matches.subcommand() {
        Some(("server", sub_m)) => {}
        Some(("gateway", sub_m)) => {}
        _ => {}
    }
}
