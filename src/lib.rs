#![feature(backtrace)]
#![feature(backtrace_frames)]
#![feature(type_name_of_val)]
#![feature(once_cell)]
#![feature(duration_constants)]
#![feature(destructuring_assignment)]
#![feature(specialization)]
#![feature(assert_matches)]
#![feature(trait_alias)]
#![feature(pattern)]
#![feature(hash_set_entry)]
#![feature(option_get_or_insert_default)]

pub mod admin;
pub mod auth;
pub mod bitrot;
pub mod bucket;
pub mod certs;
pub mod config;
pub mod crypto;
pub mod disk;
pub mod dsync;
pub mod ellipses;
pub mod encrypt;
pub mod endpoint;
pub mod erasure;
pub mod errors;
pub mod etag;
pub mod event;
pub mod format;
pub mod globals;
pub mod hash;
pub mod http;
pub mod iam;
pub mod jwt;
pub mod lock;
pub mod logger;
pub mod metacache;
pub mod mount;
pub mod net;
pub mod object;
pub mod objectcache;
pub mod pubsub;
pub mod router;
pub mod s3select;
pub mod s3utils;
pub mod serde;
pub mod signals;
pub mod storage;
pub mod strset;
pub mod tags;
pub mod trie;
pub mod utils;
pub mod version;
pub mod wildcard;
pub mod xl_storage;
