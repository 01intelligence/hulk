use std::collections::HashMap;
use std::ops::Add;

use chrono::prelude::*;
use chrono::serde::ts_seconds;
use constant_time_eq::constant_time_eq;
use jsonwebtoken::{encode, Algorithm, DecodingKey, EncodingKey, Header};
use lazy_static::lazy_static;
use rand::Rng;
use serde::Serialize;
use thiserror::Error;

use crate::jwt;
use crate::jwt::MapClaims;

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
const ALPHA_NUMERIC_TABLE: &[u8] = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".as_bytes();

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
    #[error(
        "access key length should be between {} and {}",
        ACCESS_KEY_MIN_LEN,
        ACCESS_KEY_MAX_LEN
    )]
    InvalidAccessKeyLen,
    #[error(
        "secret key length should be between {} and {}",
        SECRET_KEY_MIN_LEN,
        SECRET_KEY_MAX_LEN
    )]
    InvalidSecretKeyLen,
    #[error("invalid token expiry")]
    InvalidExpiry,
}

// Credentials holds access and secret keys.
#[derive(Serialize, Default, Debug)]
pub struct Credentials {
    #[serde(rename = "AccessKeyId")]
    pub access_key: String,
    #[serde(rename = "SecretAccessKey")]
    pub secret_key: String,
    #[serde(rename = "Expiration")]
    pub expiration: Option<DateTime<Utc>>,
    #[serde(rename = "SessionToken")]
    pub session_token: String,
    #[serde(skip)]
    pub status: String,
    #[serde(skip)]
    pub parent_user: String,
    #[serde(skip)]
    pub groups: Vec<String>,
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
        s.push(':');
        s.push_str(&self.secret_key);
        if !self.session_token.is_empty() {
            s.push('\n');
            s.push_str(&self.session_token);
        }
        if let Some(e) = &self.expiration.filter(|e| *e != *TIME_SENTINEL) {
            s.push('\n');
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

type MetaData = HashMap<String, MetaDataValue>;

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
        Err(AuthError::InvalidExpiry.into())
    } else {
        Ok(exp_at)
    }
}

pub fn generate_credentials_with_metadata(
    metadata: MetaData,
    token: &str,
) -> anyhow::Result<Credentials> {
    let mut rng = rand::thread_rng();
    let mut read_bytes = |size: usize| {
        let mut data = vec![0u8; size];
        rng.fill(&mut data[..]);
        data
    };

    let mut key_bytes = read_bytes(ACCESS_KEY_MAX_LEN);
    for b in &mut key_bytes {
        *b = ALPHA_NUMERIC_TABLE[(*b % ALPHA_NUMERIC_TABLE_LEN) as usize];
    }
    let access_key = String::from_utf8(key_bytes)?;

    let key_bytes = read_bytes(SECRET_KEY_MAX_LEN);
    let mut secret_key_str = &base64::encode(&key_bytes)[..SECRET_KEY_MAX_LEN];
    let secret_key = secret_key_str.replace("/", "+");

    new_credentials_with_metadata(access_key, secret_key, metadata, token)
}

pub fn new_credentials_with_metadata(
    access_key: String,
    secret_key: String,
    metadata: MetaData,
    token: &str,
) -> anyhow::Result<Credentials> {
    if access_key.len() < ACCESS_KEY_MIN_LEN || access_key.len() > ACCESS_KEY_MAX_LEN {
        return Err(AuthError::InvalidAccessKeyLen.into());
    }
    if secret_key.len() < SECRET_KEY_MIN_LEN || secret_key.len() > SECRET_KEY_MAX_LEN {
        return Err(AuthError::InvalidSecretKeyLen.into());
    }

    let mut cred = Credentials {
        access_key,
        secret_key,
        status: ACCOUNT_ON.into(),
        ..Default::default()
    };

    if token.is_empty() {
        cred.expiration = Some(*TIME_SENTINEL);
        return Ok(cred);
    }

    cred.session_token = jwt_sign_with_access_key(&cred.access_key, metadata, token)?;

    Ok(cred)
}

pub fn new_credentials(access_key: String, secret_key: String) -> anyhow::Result<Credentials> {
    new_credentials_with_metadata(access_key, secret_key, Default::default(), "")
}

pub fn jwt_sign_with_access_key(
    access_key: &str,
    mut metadata: MetaData,
    token: &str,
) -> anyhow::Result<String> {
    metadata.insert("accessKey".into(), MetaDataValue::String(access_key.into()));
    Ok(encode(
        &Header::new(Algorithm::HS512),
        &metadata,
        &EncodingKey::from_secret(token.as_bytes()),
    )?)
}

pub fn extract_claims(token: &str, secret_key: &str) -> anyhow::Result<MapClaims> {
    jwt::parse_with_claims(token, |_| {
        DecodingKey::from_secret(secret_key.as_bytes()).into_static()
    })
}
