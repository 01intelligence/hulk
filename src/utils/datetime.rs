use std::ops::Neg;

use chrono::{TimeZone, Utc};

pub type DateTime = chrono::DateTime<Utc>;

pub fn now() -> DateTime {
    Utc::now()
}

pub trait DateTimeExt<Tz: TimeZone> {
    fn duration_offset(self, other: chrono::DateTime<Tz>) -> std::time::Duration;
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
}
