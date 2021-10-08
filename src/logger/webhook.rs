use std::cell::RefCell;
use std::fmt::{Arguments, Write};
use std::{fmt, io, result};

use serde::ser::SerializeMap;
use serde::serde_if_integer128;
use slog::{
    o, FnValue, Key, OwnedKVList, PushFnValue, Record, SendSyncRefUnwindSafeKV, SerdeValue, KV,
};

thread_local! {
    static TL_BUF: RefCell<String> = RefCell::new(String::with_capacity(128));
}

use reqwest::{Error, Response};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use typetag::erased_serde;
use typetag::erased_serde::Serialize;

use super::Entry;
use crate::utils;

pub struct WebhookDrain {
    tx: Sender<Vec<u8>>,
    endpoint: String,
}

impl WebhookDrain {
    pub fn new(endpoint: String, user_agent: String, auth_token: Option<String>) -> Self {
        let (tx, mut rx) = channel(10000);
        let client = reqwest::Client::builder().build().unwrap(); // TODO
        let drain = WebhookDrain {
            tx,
            endpoint: endpoint.clone(),
        };

        tokio::spawn(async move {
            use http::{header, HeaderValue, StatusCode};
            while let Some(json) = rx.recv().await {
                let mut req = client
                    .post(&endpoint)
                    .timeout(utils::seconds(5))
                    .header(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    )
                    .header(
                        header::USER_AGENT,
                        HeaderValue::from_str(&user_agent).unwrap(),
                    )
                    .body(json);
                if let Some(auth_token) = &auth_token {
                    req = req.header(
                        header::AUTHORIZATION,
                        HeaderValue::from_str(auth_token).unwrap(),
                    );
                }
                match req.send().await {
                    Ok(rep) => match rep.status() {
                        StatusCode::OK => {}
                        StatusCode::FORBIDDEN => {
                            println!("{} returned '{}', please check if your auth token is correctly set", endpoint, rep.status());
                        }
                        status => {
                            println!(
                                "{} returned '{}', please check your endpoint configuration",
                                endpoint, status,
                            );
                        }
                    },
                    Err(err) => {
                        println!(
                            "{} returned '{}', please check your endpoint configuration",
                            endpoint, err,
                        );
                    }
                }
            }
        });

        drain
    }

    fn log_impl(&self, record: &Record, values: &OwnedKVList) -> std::io::Result<()> {
        let mut buf = Vec::with_capacity(128);
        {
            let mut ser = serde_json::Serializer::pretty(&mut buf);
            let mut serializer = WebhookSerializer {
                ser: Box::new(<dyn erased_serde::Serializer>::erase(&mut ser)),
            };

            // values.serialize(record, &mut serializer)?;
            record.kv().serialize(record, &mut serializer)?;
        }

        tokio::task::block_in_place(move || {
            let _ = self.tx.blocking_send(buf);
        });
        Ok(())
    }
}

impl slog::Drain for WebhookDrain {
    type Ok = ();
    type Err = slog::Never;

    fn log(&self, record: &Record, values: &OwnedKVList) -> Result<Self::Ok, Self::Err> {
        if let Err(err) = self.log_impl(record, values) {
            println!("WebhookDrain log: {}", err);
        }
        Ok(())
    }
}

struct WebhookSerializer<'a> {
    ser: Box<dyn erased_serde::Serializer + 'a>,
}

impl<'a> slog::Serializer for WebhookSerializer<'a> {
    fn emit_arguments(&mut self, key: Key, val: &Arguments) -> slog::Result {
        // Deny any value, excluding `SerdeValue`.
        Err(slog::Error::Other)
    }

    fn emit_serde(&mut self, key: Key, value: &SerdeValue) -> slog::Result {
        value
            .as_serde()
            .erased_serialize(&mut self.ser)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("serde serialization error: {}", e),
                )
            })?;
        Ok(())
    }
}

/// `slog::Serializer` adapter for `serde::Serializer`
///
/// Newtype to wrap serde Serializer, so that `Serialize` can be implemented
/// for it
struct SerdeSerializer<S: serde::Serializer> {
    /// Current state of map serializing: `serde::Serializer::MapState`
    ser_map: S::SerializeMap,
}

impl<S: serde::Serializer> SerdeSerializer<S> {
    /// Start serializing map of values
    fn start(ser: S, len: Option<usize>) -> result::Result<Self, slog::Error> {
        let ser_map = ser.serialize_map(len).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("serde serialization error: {}", e),
            )
        })?;
        Ok(SerdeSerializer { ser_map })
    }

    /// Finish serialization, and return the serializer
    fn end(self) -> result::Result<S::Ok, S::Error> {
        self.ser_map.end()
    }
}

macro_rules! impl_m(
    ($s:expr, $key:expr, $val:expr) => ({
        let k_s:  &str = $key.as_ref();
        $s.ser_map.serialize_entry(k_s, $val)
             .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("serde serialization error: {}", e)))?;
        Ok(())
    });
);

impl<S> slog::Serializer for SerdeSerializer<S>
where
    S: serde::Serializer,
{
    fn emit_bool(&mut self, key: Key, val: bool) -> slog::Result {
        impl_m!(self, key, &val)
    }

    fn emit_unit(&mut self, key: Key) -> slog::Result {
        impl_m!(self, key, &())
    }

    fn emit_char(&mut self, key: Key, val: char) -> slog::Result {
        impl_m!(self, key, &val)
    }

    fn emit_none(&mut self, key: Key) -> slog::Result {
        let val: Option<()> = None;
        impl_m!(self, key, &val)
    }
    fn emit_u8(&mut self, key: Key, val: u8) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i8(&mut self, key: Key, val: i8) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_u16(&mut self, key: Key, val: u16) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i16(&mut self, key: Key, val: i16) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_usize(&mut self, key: Key, val: usize) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_isize(&mut self, key: Key, val: isize) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_u32(&mut self, key: Key, val: u32) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i32(&mut self, key: Key, val: i32) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_f32(&mut self, key: Key, val: f32) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_u64(&mut self, key: Key, val: u64) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_i64(&mut self, key: Key, val: i64) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_f64(&mut self, key: Key, val: f64) -> slog::Result {
        impl_m!(self, key, &val)
    }
    serde_if_integer128! {
        fn emit_u128(&mut self, key: Key, val: u128) -> slog::Result {
            impl_m!(self, key, &val)
        }
        fn emit_i128(&mut self, key: Key, val: i128) -> slog::Result {
            impl_m!(self, key, &val)
        }
    }
    fn emit_str(&mut self, key: Key, val: &str) -> slog::Result {
        impl_m!(self, key, &val)
    }
    fn emit_arguments(&mut self, key: Key, val: &fmt::Arguments) -> slog::Result {
        TL_BUF.with(|buf| {
            let mut buf = buf.borrow_mut();

            buf.write_fmt(*val).unwrap();

            let res = { || impl_m!(self, key, &*buf) }();
            buf.clear();
            res
        })
    }

    fn emit_serde(&mut self, key: Key, value: &dyn slog::SerdeValue) -> slog::Result {
        self.ser_map.serialize_entry(key, value.as_serde());
        impl_m!(self, key, value.as_serde())
    }
}

/// Json `Drain`
///
/// Each record will be printed as a Json map
/// to a given `io`
pub struct Json<W: io::Write> {
    newlines: bool,
    flush: bool,
    values: Vec<OwnedKVList>,
    io: RefCell<W>,
    pretty: bool,
}

impl<W> Json<W>
where
    W: io::Write,
{
    /// New `Json` `Drain` with default key-value pairs added
    pub fn default(io: W) -> Json<W> {
        JsonBuilder::new(io).add_default_keys().build()
    }

    /// Build custom `Json` `Drain`
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::new_ret_no_self))]
    pub fn new(io: W) -> JsonBuilder<W> {
        JsonBuilder::new(io)
    }

    fn log_impl<F>(
        &self,
        serializer: &mut serde_json::ser::Serializer<&mut W, F>,
        rinfo: &Record,
        logger_values: &OwnedKVList,
    ) -> io::Result<()>
    where
        F: serde_json::ser::Formatter,
    {
        let mut serializer = SerdeSerializer::start(&mut *serializer, None)?;

        for kv in &self.values {
            kv.serialize(rinfo, &mut serializer)?;
        }

        logger_values.serialize(rinfo, &mut serializer)?;

        rinfo.kv().serialize(rinfo, &mut serializer)?;

        let res = serializer.end();

        res.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(())
    }
}

impl<W> slog::Drain for Json<W>
where
    W: io::Write,
{
    type Ok = ();
    type Err = io::Error;
    fn log(&self, rinfo: &Record, logger_values: &OwnedKVList) -> io::Result<()> {
        let mut io = self.io.borrow_mut();
        let io = if self.pretty {
            let mut serializer = serde_json::Serializer::pretty(&mut *io);
            self.log_impl(&mut serializer, rinfo, logger_values)?;
            serializer.into_inner()
        } else {
            let mut serializer = serde_json::Serializer::new(&mut *io);
            self.log_impl(&mut serializer, rinfo, logger_values)?;
            serializer.into_inner()
        };
        if self.newlines {
            io.write_all("\n".as_bytes())?;
        }
        if self.flush {
            io.flush()?;
        }
        Ok(())
    }
}

/// Json `Drain` builder
///
/// Create with `Json::new`.
pub struct JsonBuilder<W: io::Write> {
    newlines: bool,
    flush: bool,
    values: Vec<OwnedKVList>,
    io: W,
    pretty: bool,
}

impl<W> JsonBuilder<W>
where
    W: io::Write,
{
    fn new(io: W) -> Self {
        JsonBuilder {
            newlines: true,
            flush: false,
            values: vec![],
            io,
            pretty: false,
        }
    }

    /// Build `Json` `Drain`
    ///
    /// This consumes the builder.
    pub fn build(self) -> Json<W> {
        Json {
            values: self.values,
            newlines: self.newlines,
            flush: self.flush,
            io: RefCell::new(self.io),
            pretty: self.pretty,
        }
    }

    /// Set writing a newline after every log record
    pub fn set_newlines(mut self, enabled: bool) -> Self {
        self.newlines = enabled;
        self
    }

    /// Enable flushing of the `io::Write` after every log record
    pub fn set_flush(mut self, enabled: bool) -> Self {
        self.flush = enabled;
        self
    }

    /// Set whether or not pretty formatted logging should be used
    pub fn set_pretty(mut self, enabled: bool) -> Self {
        self.pretty = enabled;
        self
    }

    /// Add custom values to be printed with this formatter
    pub fn add_key_value<T>(mut self, value: slog::OwnedKV<T>) -> Self
    where
        T: SendSyncRefUnwindSafeKV + 'static,
    {
        self.values.push(value.into());
        self
    }

    /// Add default key-values:
    ///
    /// * `ts` - timestamp
    /// * `level` - record logging level name
    /// * `msg` - msg - formatted logging message
    pub fn add_default_keys(self) -> Self {
        self.add_key_value(o!(
            "ts" => PushFnValue(move |_ : &Record, ser| {
                ser.emit(chrono::Local::now().to_rfc3339())
            }),
            "level" => FnValue(move |rinfo : &Record| {
                rinfo.level().as_short_str()
            }),
            "msg" => PushFnValue(move |record : &Record, ser| {
                ser.emit(record.msg())
            }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn test_logger_webhook() {
        use actix_web::{web, App, HttpRequest, HttpServer, Responder};
        use slog::{info, Drain};

        async fn index(json: String) -> impl Responder {
            println!("Request body: '{}'", json);
            json
        }

        actix_rt::spawn(async {
            HttpServer::new(|| App::new().route("/", web::post().to(index)))
                .bind(("127.0.0.1", 8080))
                .unwrap()
                .run()
                .await;
        });

        let drain = WebhookDrain::new("http://127.0.0.1:8080".to_owned(), "".to_owned(), None);
        let drain = slog_async::Async::new(drain).build().fuse();
        let log = slog::Logger::root(drain, slog::slog_o!());

        use super::super::log;
        let entry = log::Entry {
            deployment_id: "deployment_id".to_string(),
            level: "level".to_string(),
            kind: super::super::ErrKind::System,
            time: "time".to_string(),
            api: Some(log::Api {
                name: "name".to_string(),
                args: Some(log::Args {
                    bucket: "bucket".to_string(),
                    object: "object".to_string(),
                    metadata: Default::default(),
                }),
            }),
            remote_host: "remote_host".to_string(),
            host: "host".to_string(),
            request_id: "request_id".to_string(),
            user_agent: "user_agent".to_string(),
            message: "message".to_string(),
            error: None,
        };
        slog::info!(log, ""; entry);

        tokio::time::sleep(utils::milliseconds(200)).await;
    }
}
