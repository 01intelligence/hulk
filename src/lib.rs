#![feature(backtrace)]
#![feature(backtrace_frames)]
#![feature(type_name_of_val)]
#![feature(once_cell)]
#![feature(duration_constants)]
#![feature(destructuring_assignment)]
#![feature(specialization)]
#![feature(assert_matches)]

pub mod auth;
pub mod bitrot;
pub mod bucket;
pub mod certs;
pub mod config;
pub mod disk;
pub mod dsync;
pub mod ellipses;
pub mod endpoint;
pub mod erasure;
pub mod errors;
pub mod etag;
pub mod format;
pub mod globals;
pub mod hash;
pub mod http;
pub mod iam;
pub mod jwt;
pub mod lock;
pub mod log;
pub mod mount;
pub mod net;
pub mod object;
pub mod s3utils;
pub mod signals;
pub mod strset;
pub mod trie;
pub mod utils;
pub mod version;
pub mod wildcard;
