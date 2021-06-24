#![feature(backtrace)]
#![feature(backtrace_frames)]
#![feature(type_name_of_val)]
#![feature(once_cell)]

pub mod auth;
pub mod bucket;
pub mod certs;
pub mod config;
pub mod disk;
pub mod dsync;
pub mod ellipses;
pub mod erasure;
pub mod etag;
pub mod iam;
pub mod jwt;
pub mod log;
pub mod s3utils;
pub mod strset;
pub mod trie;
pub mod utils;
pub mod version;
pub mod wildcard;
