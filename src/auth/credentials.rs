use std::ops::Add;

use chrono::prelude::*;
use chrono::serde::ts_seconds;
use constant_time_eq::constant_time_eq;
use lazy_static::lazy_static;
use serde::Serialize;
use thiserror::Error;

// Minimum length for Hulk access key.
const ACCESS_KEY_MIN_LEN: usize = 3;

// Maximum length for Hulk access key.
// There is no max length enforcement for access keys
const ACCESS_KEY_MAX_LEN: usize = 20;

// Minimum length for Hulk secret key for both server and gateway mode.
const SECRET_KEY_MIN_LEN: usize = 8;

// Maximum secret key length for Hulk, this
// is used when auto-generating new credentials.
// There is no max length enforcement for secret keys
const SECRET_KEY_MAX_LEN: usize = 40;

// Alpha numeric table used for generating access keys.
const ALPHA_NUMERIC_TABLE: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

// Total length of the alpha numeric table.
const ALPHA_NUMERIC_TABLE_LEN: u8 = ALPHA_NUMERIC_TABLE.len() as u8;

pub const DEFAULT_ACCESS_KEY: &str = "hulkadmin";
pub const DEFAULT_SECRET_KEY: &str = "hulkadmin";

pub fn is_access_key_valid(access_key: &str) -> bool {
    access_key.len() >= ACCESS_KEY_MIN_LEN
}

pub fn is_secret_key_valid(secret_key: &str) -> bool {
    secret_key.len() >= SECRET_KEY_MIN_LEN
}

lazy_static! {
    static ref TIME_SENTINEL: DateTime<Utc> =
        DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc);
}
const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.9f %z %Z";

// ACCOUNT_ON indicates that credentials are enabled
pub const ACCOUNT_ON: &str = "on";
// ACCOUNT_OFF indicates that credentials are disabled
pub const ACCOUNT_OFF: &str = "off";

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("invalid token expiry")]
    InvalidExpiry,
}

// Credentials holds access and secret keys.
#[derive(Serialize, Debug)]
struct Credentials {
    #[serde(rename = "AccessKeyId")]
    access_key: String,
    #[serde(rename = "SecretAccessKey")]
    secret_key: String,
    #[serde(rename = "Expiration")]
    expiration: Option<DateTime<Utc>>,
    #[serde(rename = "SessionToken")]
    session_token: String,
    #[serde(skip)]
    status: String,
    #[serde(skip)]
    parent_user: String,
    #[serde(skip)]
    groups: Vec<String>,
}

impl Credentials {
    pub fn is_expired(&self) -> bool {
        if datetime_is_zero(&self.expiration) {
            return false;
        }
        self.expiration.unwrap().lt(&Utc::now())
    }

    pub fn is_temp(&self) -> bool {
        !self.session_token.is_empty() && !datetime_is_zero(&self.expiration)
    }

    pub fn is_service_account(&self) -> bool {
        !self.parent_user.is_empty() && datetime_is_zero(&self.expiration)
    }

    pub fn is_valid(&self) -> bool {
        if self.status == ACCOUNT_OFF {
            return false;
        }
        is_access_key_valid(&self.access_key)
            && is_secret_key_valid(&self.secret_key)
            && !self.is_expired()
    }
}

fn datetime_is_zero(dt: &Option<DateTime<Utc>>) -> bool {
    dt.filter(|e| *e != *TIME_SENTINEL).is_none()
}

impl PartialEq for Credentials {
    fn eq(&self, other: &Self) -> bool {
        if !self.is_valid() {
            return false;
        }
        self.access_key == other.access_key
            && constant_time_eq(self.secret_key.as_bytes(), other.secret_key.as_bytes())
            && constant_time_eq(
                self.session_token.as_bytes(),
                other.session_token.as_bytes(),
            )
    }
}

impl ToString for Credentials {
    fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&self.access_key);
        s.push_str(":");
        s.push_str(&self.secret_key);
        if !self.session_token.is_empty() {
            s.push_str("\n");
            s.push_str(&self.session_token);
        }
        if let Some(e) = &self.expiration.filter(|e| *e != *TIME_SENTINEL) {
            s.push_str("\n");
            s.push_str(&e.format(TIME_FORMAT).to_string());
        }
        s
    }
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum MetaDataValue {
    String(String),
    I64(i64),
    U64(u64),
    Isize(isize),
    Usize(usize),
    F64(f64),
}

pub fn exp_to_int64(exp: Option<MetaDataValue>) -> anyhow::Result<i64> {
    use MetaDataValue::*;
    let exp_at = if let Some(exp) = exp {
        match exp {
            String(v) => v.parse::<i64>().map_err(|_| AuthError::InvalidExpiry)?,
            Isize(v) => v as i64,
            Usize(v) => v as i64,
            I64(v) => v as i64,
            U64(v) => v as i64,
            F64(v) => v as i64,
        }
    } else {
        0
    };
    if exp_at < 0 {
        Err(AuthError::InvalidExpiry)?
    } else {
        Ok(exp_at)
    }
}
