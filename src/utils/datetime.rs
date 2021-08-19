use std::ops::Neg;

use chrono::{TimeZone, Utc};

pub type DateTime = chrono::DateTime<Utc>;

pub fn now() -> DateTime {
    Utc::now()
}

pub const MIN_DATETIME: DateTime = chrono::MIN_DATETIME;

pub trait DateTimeExt<Tz: TimeZone> {
    fn duration_offset(self, other: chrono::DateTime<Tz>) -> std::time::Duration;
    fn is_min(&self) -> bool;
    fn from_timestamp_nanos(nanos_since_unix_epoch: i64) -> Self;
}

impl DateTimeExt<Utc> for DateTime {
    fn duration_offset(self, other: chrono::DateTime<Utc>) -> std::time::Duration {
        let offset = self.signed_duration_since(other);
        if offset < chrono::Duration::zero() {
            offset.neg().to_std().unwrap()
        } else {
            offset.to_std().unwrap()
        }
    }

    fn is_min(&self) -> bool {
        self == &chrono::MIN_DATETIME
    }

    fn from_timestamp_nanos(nanos_since_unix_epoch: i64) -> Self {
        let secs = nanos_since_unix_epoch / 1_000_000_000;
        let nanos = nanos_since_unix_epoch - secs * 1_000_000_000;
        // Safety: `secs` and `nanos` are both valid.
        let native = chrono::NaiveDateTime::from_timestamp(secs, nanos as u32);
        DateTime::from_utc(native, Utc)
    }
}
