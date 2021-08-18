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
}
