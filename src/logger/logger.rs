use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use highway::HighwayHash;
use lazy_static::lazy_static;
use opentelemetry::Context;

use crate::globals::{ReadWriteGuard, GLOBALS};
use crate::logger::backtrace::Inner::*;
use crate::logger::backtrace::{Backtrace, BytesOrWide};
use crate::logger::entry::log::{Api, Args, Entry, Trace};
use crate::logger::entry::ErrKind;
use crate::logger::reqinfo::ReqInfoContextExt;
use crate::utils;
use crate::utils::DateTimeFormatExt;

// HighwayHash key for logging in anonymous mode
const MAGIC_HIGHWAY_HASH_256_KEY: [u8; 32] =
    hex_literal::hex!("4be734fa8e238acd263e83e6bb968552040f935da39f441497e09d1322de36a0");

lazy_static! {
    pub(super) static ref QUIET_FLAG: bool = GLOBALS.cli_context.guard().quiet;
    pub(super) static ref ANONYMOUS_FLAG: bool = GLOBALS.cli_context.guard().anonymous;
    pub(super) static ref JSON_FLAG: bool = GLOBALS.cli_context.guard().json;
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

pub fn log<Err: std::error::Error>(ctx: Context, err: Err) {
    log_inner(ctx, err, None);
}

pub fn log_with_kind<Err: std::error::Error>(ctx: Context, err: Err, err_kind: Option<ErrKind>) {
    log_inner(ctx, err, err_kind);
}

fn log_inner<Err: std::error::Error>(ctx: Context, err: Err, err_kind: Option<ErrKind>) {
    let err_kind = err_kind.unwrap_or(ErrKind::System);
    let req = ctx.req_info();

    let api = if req.api.is_empty() {
        "SYSTEM".to_string()
    } else {
        req.api.clone()
    };

    let tags = req.get_tags_map();

    let trace = get_trace(4);

    let message = format!("{} ({})", err, std::any::type_name_of_val(&err));

    let deployment_id = if req.deployment_id.is_empty() {
        GLOBALS.deployment_id.guard().clone()
    } else {
        req.deployment_id.clone()
    };

    let mut entry = Entry {
        deployment_id,
        level: Level::Error.to_string(),
        kind: err_kind,
        time: utils::now().rfc3339_nano(),
        api: Api {
            name: api,
            args: Some(Args {
                bucket: req.bucket_name.clone(),
                object: req.object_name.clone(),
                metadata: Default::default(),
            }),
        },
        remote_host: req.remote_host.clone(),
        host: req.host.clone(),
        request_id: req.request_id.clone(),
        user_agent: req.user_agent.clone(),
        message: "".to_string(),
        trace: Trace {
            message,
            source: trace,
            variables: tags,
        },
    };

    if *ANONYMOUS_FLAG {
        let args = entry.api.args.as_mut().unwrap();
        args.bucket = hash_string(&args.bucket);
        args.object = hash_string(&args.object);
        entry.remote_host = hash_string(&entry.remote_host);
        entry.trace.message = std::any::type_name_of_val(&err).to_owned();
        entry.trace.variables = Default::default();
    }

    slog::error!(super::LOG_LOGGER, ""; entry);
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

    let mut trace = Vec::new();
    for f in frames.iter().skip(trace_level) {
        if f.frame.ip().is_null() {
            continue;
        }
        for symbol in f.symbols.iter() {
            let symbol_name = symbol.name.as_ref().map(|b| backtrace::SymbolName::new(b));
            let file_name = symbol.filename.as_ref().map(|b| match b {
                BytesOrWide::Bytes(w) => backtrace::BytesOrWideString::Bytes(w).into_path_buf(),
                BytesOrWide::Wide(w) => backtrace::BytesOrWideString::Wide(w).into_path_buf(),
            });

            use std::fmt::Write;
            let mut s = String::new();
            if let (Some(file_name), Some(lineno)) = (file_name, symbol.lineno) {
                write!(s, "{:?}:{}:", file_name, lineno);
                if let Some(colno) = symbol.colno {
                    write!(s, "{}:", colno);
                }
            }
            if let Some(symbol_name) = symbol_name {
                write!(s, "{}", symbol_name);
            } else {
                write!(s, "<unknown>");
            }
            trace.push(s);

            // TODO: ignore backtrace symbols beyond the following conditions.
        }
    }
    trace
}

#[macro_export]
macro_rules! trace(
    ($($args:tt)*) => {
        slog::trace!($crate::logger::INTRINSIC_LOGGER, $($args)*)
    };
);

#[macro_export]
macro_rules! info(
    ($($args:tt)*) => {
        slog::info!($crate::logger::INTRINSIC_LOGGER, $($args)*)
    };
);

#[macro_export]
macro_rules! warn(
    ($($args:tt)*) => {
        slog::warn!($crate::logger::INTRINSIC_LOGGER, $($args)*)
    };
);

#[macro_export]
macro_rules! error(
    ($($args:tt)*) => {
        slog::error!($crate::logger::INTRINSIC_LOGGER, $($args)*)
    };
);

#[macro_export]
macro_rules! fatal(
    ($($args:tt)+) => {
        slog::crit!($crate::logger::INTRINSIC_LOGGER, $($args)+);
        std::process::exit(1)
    };
);
