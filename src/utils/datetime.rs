pub use std::time::Duration;

use chrono::{FixedOffset, TimeZone, Timelike, Utc};
use lazy_static::lazy_static;

pub type DateTime = chrono::DateTime<Utc>;
pub type ChronoDuration = chrono::Duration;

lazy_static! {
    pub static ref UNIX_EPOCH: DateTime = Utc.timestamp(0, 0);
}

pub const MIN_DATETIME: DateTime = chrono::MIN_DATETIME;

pub fn now() -> DateTime {
    Utc::now()
}

pub trait DateTimeExt<Tz: TimeZone> {
    /// Returns the amount of time elapsed since this datetime was created.
    fn elapsed(self) -> Duration;
    /// Returns the amount of time elapsed from another datetime to this one,
    /// or zero duration if that instant is later than this one.
    fn duration_since(self, earlier: chrono::DateTime<Tz>) -> Duration;
    /// Returns the amount of time between another datetime and this one,
    fn duration_offset(self, other: chrono::DateTime<Tz>) -> Duration;
    fn is_min(&self) -> bool;
    fn from_timestamp_nanos(nanos_since_unix_epoch: i64) -> Self;
}

impl DateTimeExt<Utc> for DateTime {
    fn elapsed(self) -> Duration {
        now().duration_since(self)
    }

    fn duration_since(self, earlier: chrono::DateTime<Utc>) -> Duration {
        self.signed_duration_since(earlier)
            .to_std()
            .unwrap_or_else(|_| Duration::ZERO)
    }

    fn duration_offset(self, other: chrono::DateTime<Utc>) -> Duration {
        let offset = self.signed_duration_since(other);
        if offset < chrono::Duration::zero() {
            use std::ops::Neg;
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

pub trait DateTimeFormatExt {
    fn fmt_to(&self, fmt: &str) -> String;
    fn rfc3339(&self) -> String;
    fn rfc3339_nano(&self) -> String;
    fn parse(s: &str, fmt: &str) -> anyhow::Result<Self>
    where
        Self: Sized;
    fn from_rfc3339(s: &str) -> anyhow::Result<Self>
    where
        Self: Sized;
    fn from_rfc3339_nano(s: &str) -> anyhow::Result<Self>
    where
        Self: Sized;
}

impl DateTimeFormatExt for chrono::DateTime<FixedOffset> {
    fn fmt_to(&self, fmt: &str) -> String {
        self.format(fmt).to_string()
    }

    fn rfc3339(&self) -> String {
        self.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    fn rfc3339_nano(&self) -> String {
        self.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
    }

    fn parse(s: &str, fmt: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self::parse_from_str(s, fmt)?)
    }

    fn from_rfc3339(s: &str) -> anyhow::Result<Self> {
        let dt = chrono::DateTime::parse_from_rfc3339(s)?;
        if dt.nanosecond() > 0 {
            return Err(anyhow::anyhow!("input contains invalid characters"));
        }
        Ok(dt)
    }

    fn from_rfc3339_nano(s: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(chrono::DateTime::parse_from_rfc3339(s)?)
    }
}

impl DateTimeFormatExt for chrono::DateTime<Utc> {
    fn fmt_to(&self, fmt: &str) -> String {
        self.format(fmt).to_string()
    }

    fn rfc3339(&self) -> String {
        self.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    fn rfc3339_nano(&self) -> String {
        self.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
    }

    fn parse(s: &str, fmt: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let dt = chrono::DateTime::parse_from_str(s, fmt)?;
        Ok(dt.with_timezone(&Utc))
    }

    fn from_rfc3339(s: &str) -> anyhow::Result<Self> {
        let dt = chrono::DateTime::parse_from_rfc3339(s)?;
        if dt.nanosecond() > 0 {
            return Err(anyhow::anyhow!("input contains invalid characters"));
        }
        Ok(dt.with_timezone(&Utc))
    }

    fn from_rfc3339_nano(s: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let dt = chrono::DateTime::parse_from_rfc3339(s)?;
        Ok(dt.with_timezone(&Utc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_format_and_parse() {
        let rfc3339_cases = vec![
            (
                Utc.ymd(2008, 9, 17)
                    .and_hms(20, 4, 26)
                    .with_timezone(&FixedOffset::east(0)),
                "2008-09-17T20:04:26Z",
            ),
            (
                FixedOffset::east(-18000)
                    .ymd(1994, 9, 17)
                    .and_hms(20, 4, 26),
                "1994-09-17T20:04:26-05:00",
            ),
            (
                FixedOffset::east(15600).ymd(2000, 12, 26).and_hms(1, 15, 6),
                "2000-12-26T01:15:06+04:20",
            ),
        ];
        for (ref dt, s) in rfc3339_cases {
            let got = dt.rfc3339();
            assert_eq!(got, s, "RFC3339: want '{}', got '{}'", s, got);
            let got_dt = chrono::DateTime::<FixedOffset>::from_rfc3339(&got).unwrap();
            assert_eq!(&got_dt, dt, "RFC3339: want '{}', got '{}'", dt, got_dt);
        }

        let rfc3339_nano_cases = vec![(
            FixedOffset::east(-18000)
                .ymd(1994, 9, 17)
                .and_hms_nano(20, 4, 26, 12345600),
            "1994-09-17T20:04:26.012345600-05:00",
        )];
        for (ref dt, s) in rfc3339_nano_cases {
            let got = dt.rfc3339_nano();
            assert_eq!(got, s, "RFC3339: want '{}', got '{}'", s, got);
            assert!(chrono::DateTime::<FixedOffset>::from_rfc3339(&got).is_err());
            let got_dt = chrono::DateTime::<FixedOffset>::from_rfc3339_nano(&got).unwrap();
            assert_eq!(&got_dt, dt, "RFC3339: want '{}', got '{}'", dt, got_dt);
        }
    }
}
