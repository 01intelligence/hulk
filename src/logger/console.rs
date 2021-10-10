use std::fmt::Arguments;

use slog::{Drain, Key, OwnedKVList, Record, KV};
use slog_term::{Decorator, RecordDecorator};

pub struct ConsoleDrain {
    decorator: slog_term::TermDecorator,
}

impl ConsoleDrain {
    pub fn new() -> Self {
        ConsoleDrain {
            decorator: slog_term::TermDecorator::new().build(),
        }
    }

    pub fn target(&self) -> super::Target {
        super::Target {
            name: "console".to_owned(),
            endpoint: None,
        }
    }
}

impl Drain for ConsoleDrain {
    type Ok = ();
    type Err = std::io::Error;

    fn log(&self, record: &Record, values: &OwnedKVList) -> std::io::Result<()> {
        self.decorator.with_record(record, values, |decorator| {
            let mut serializer = ConsoleSerializer { decorator };

            record.kv().serialize(record, &mut serializer)?;
            values.serialize(record, &mut serializer)?;

            decorator.start_whitespace()?;
            writeln!(decorator)?;
            decorator.flush()?;
            Ok(())
        })
    }
}

struct ConsoleSerializer<'a> {
    decorator: &'a mut dyn RecordDecorator,
}

impl<'a> slog::Serializer for ConsoleSerializer<'a> {
    fn emit_arguments(&mut self, key: Key, val: &Arguments) -> slog::Result {
        // Deny any value, excluding `SerdeValue`.
        Err(slog::Error::Other)
    }

    fn emit_serde(&mut self, key: Key, value: &slog::SerdeValue) -> slog::Result {
        let entry = value.as_any().downcast_ref::<super::log::Entry>().unwrap();

        self.decorator.start_whitespace()?;

        if *super::JSON_FLAG {
            write!(
                self.decorator,
                "{}",
                serde_json::to_string(entry).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("serde serialization error: {}", e),
                    )
                })?
            )?;
            return Ok(());
        }

        write!(self.decorator, "API: {}(", entry.api.name)?;
        if let Some(args) = &entry.api.args {
            if !args.bucket.is_empty() {
                write!(
                    self.decorator,
                    "bucket={}, object={}",
                    args.bucket, args.object
                )?;
            }
        }
        write!(self.decorator, ")")?;

        write!(self.decorator, "\nTime: ")?;
        self.decorator.start_timestamp()?;
        slog_term::timestamp_local(self.decorator)?;

        self.decorator.start_whitespace()?;

        if !entry.deployment_id.is_empty() {
            write!(self.decorator, "\nDeploymentID: {}", entry.deployment_id)?;
        }

        if !entry.request_id.is_empty() {
            write!(self.decorator, "\nRequestID: {}", entry.request_id)?;
        }

        if !entry.remote_host.is_empty() {
            write!(self.decorator, "\nRemoteHost: {}", entry.remote_host)?;
        }

        if !entry.host.is_empty() {
            write!(self.decorator, "\nHost: {}", entry.host)?;
        }

        if !entry.user_agent.is_empty() {
            write!(self.decorator, "\nUserAgent: {}", entry.user_agent)?;
        }

        self.decorator.start_msg()?;
        write!(self.decorator, "\nError: {}", entry.trace.message)?;

        let mut tag_started = false;
        for (key, val) in &entry.trace.variables {
            if let super::log::Value::String(val) = val {
                if !tag_started {
                    write!(self.decorator, "\n       ")?;
                } else {
                    write!(self.decorator, ", ")?;
                }
                write!(self.decorator, "{}={}", key, val)?;
                tag_started = true;
            }
        }

        for (i, source) in entry.trace.source.iter().enumerate() {
            write!(
                self.decorator,
                "\n{:>8}: {}",
                entry.trace.source.len() - i,
                source
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn test_logger_console() {
        let drain = ConsoleDrain::new().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let log = slog::Logger::root(drain, slog::slog_o!());

        use super::super::log;
        let entry = log::Entry {
            deployment_id: "deployment_id".to_string(),
            level: "level".to_string(),
            kind: super::super::ErrKind::System,
            time: "time".to_string(),
            api: log::Api {
                name: "name".to_string(),
                args: Some(log::Args {
                    bucket: "bucket".to_string(),
                    object: "object".to_string(),
                    metadata: Default::default(),
                }),
            },
            remote_host: "remote_host".to_string(),
            host: "host".to_string(),
            request_id: "request_id".to_string(),
            user_agent: "user_agent".to_string(),
            message: "message".to_string(),
            trace: log::Trace {
                message: "message".to_string(),
                source: vec!["source one".to_string(), "source two".to_string()],
                variables: maplit::hashmap! {
                    "k1".to_owned() => log::Value::String("v1".into()),
                    "k2".to_owned() => log::Value::String("v2".into()),
                },
            },
        };
        slog::info!(log, ""; entry);
    }
}
