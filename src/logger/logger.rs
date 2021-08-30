use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use highway::HighwayHash;
use lazy_static::lazy_static;
use log::{error, info, warn};
use opentelemetry::Context;

use crate::logger::backtrace::Backtrace;
use crate::logger::backtrace::Inner::*;
use crate::logger::entry::{Api, Args, Entry, ErrKind, Trace};
use crate::logger::reqinfo::ReqInfoContextExt;
use crate::utils;
use crate::utils::DateTimeFormatExt;

// HighwayHash key for logging in anonymous mode
const MAGIC_HIGHWAY_HASH_256_KEY: [u8; 32] =
    hex_literal::hex!("4be734fa8e238acd263e83e6bb968552040f935da39f441497e09d1322de36a0");

lazy_static! {
    static ref GLOBAL_DEPLOYMENT_ID: RwLock<String> = RwLock::new("".to_string());
    static ref ANONYMOUS_FLAG: AtomicBool = AtomicBool::new(false);
    static ref LOGGER_HIGHWAY_KEY: highway::Key = {
        let mut key = [0; 4];
        let mut rdr = std::io::Cursor::new(MAGIC_HIGHWAY_HASH_256_KEY);
        rdr.read_u64_into::<LittleEndian>(&mut key).unwrap();
        highway::Key(key)
    };
}

#[derive(strum::ToString, Debug)]
enum Level {
    Info,
    Error,
    Fatal,
}

// SetDeploymentID -- Deployment Id from the main package is set here
pub fn set_deployment_id(deployment_id: String) {
    let mut id = GLOBAL_DEPLOYMENT_ID.write().unwrap();
    *id = deployment_id;
}

fn hash_string(input: &str) -> String {
    let mut hasher = highway::HighwayHasher::new(*LOGGER_HIGHWAY_KEY);
    hasher.append(input.as_bytes());
    let hash = hasher.finalize256();
    let mut wdr = std::io::Cursor::new(vec![0u8; 32]);
    hash.iter().for_each(|item| {
        wdr.write_u64::<LittleEndian>(*item).unwrap();
    });
    hex::encode(wdr.get_ref())
}

fn log_if<Err: std::error::Error>(ctx: Context, err: Err, err_kind: Option<ErrKind>) {
    let err_kind = err_kind.unwrap_or(ErrKind::Hulk);
    let req = ctx.req_info();

    let api = if req.api.is_empty() {
        "SYSTEM".to_string()
    } else {
        req.api.clone()
    };

    let tags = req.get_tags_map();

    let trace = get_trace(3);

    let message = format!("{} ({})", err, std::any::type_name_of_val(&err));

    let deployment_id = if req.deployment_id.is_empty() {
        GLOBAL_DEPLOYMENT_ID.read().unwrap().clone()
    } else {
        req.deployment_id.clone()
    };

    let mut entry = Entry {
        deployment_id,
        level: Level::Error.to_string(),
        log_kind: err_kind.to_string(),
        time: utils::now().rfc3339_nano(),
        api: Some(Api {
            name: api,
            args: Some(Args {
                bucket: req.bucket_name.clone(),
                object: req.object_name.clone(),
                metadata: Default::default(),
            }),
        }),
        remote_host: "".to_string(),
        host: "".to_string(),
        request_id: "".to_string(),
        user_agent: "".to_string(),
        message: "".to_string(),
        trace: Some(Trace {
            message,
            source: trace,
            variables: tags,
        }),
    };

    if ANONYMOUS_FLAG.load(Ordering::Relaxed) {
        let api = entry.api.as_mut().unwrap();
        let args = api.args.as_mut().unwrap();
        args.bucket = hash_string(&args.bucket);
        args.object = hash_string(&args.object);
        entry.remote_host = hash_string(&entry.remote_host);
    }

    error!("{:?}", entry);
}

// Creates and returns stack trace
fn get_trace(trace_level: usize) -> Vec<String> {
    let bt = Backtrace::capture();
    let capture = match &bt.inner {
        Unsupported => {
            return vec!["<unsupported>".to_string()];
        }
        Disabled => {
            return vec!["<disabled>".to_string()];
        }
        Captured(c) => c.force(),
    };

    let frames = &capture.frames[capture.actual_start..];

    let trace = Vec::new();
    for f in frames.iter().skip(trace_level) {
        if f.frame.ip().is_null() {
            continue;
        }
        // TODO:
    }
    trace
}
