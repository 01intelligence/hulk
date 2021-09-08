use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::utils;

#[derive(Error, Debug)]
pub enum JwtError {
    #[error("invalid audience")]
    Audience,
    #[error("token is expired")]
    Expired,
    #[error("token used before issued")]
    IssuedAt,
    #[error("invalid issuer")]
    Issuer,
    #[error("token is not valid yet")]
    NotValidYet,
    #[error("invalid jti")]
    Id,
    #[error("{0}")]
    Other(String),
    #[error("{0:?}")]
    Aggregated(Vec<JwtError>),
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct StandardClaims {
    #[serde(rename = "accessKey")]
    pub access_key: String,

    #[serde(rename = "aud", skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>, // Audience
    #[serde(rename = "exp", skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<usize>, // Expiration time (as UTC timestamp)
    #[serde(rename = "jwt", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>, // JWT ID
    #[serde(rename = "iat", skip_serializing_if = "Option::is_none")]
    pub issued_at: Option<usize>, // Issued at (as UTC timestamp)
    #[serde(rename = "iss", skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>, // Issuer
    #[serde(rename = "nbf", skip_serializing_if = "Option::is_none")]
    pub not_before: Option<usize>, // Not Before (as UTC timestamp)
    #[serde(rename = "sub")]
    pub subject: String, // Subject (whom token refers to)
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct MapClaims(serde_json::value::Value);

impl StandardClaims {
    pub fn new() -> StandardClaims {
        Default::default()
    }

    pub fn set_issuer(&mut self, issuer: &str) {
        self.issuer.insert(issuer.into());
    }

    pub fn set_audience(&mut self, audience: &str) {
        self.audience.insert(audience.into());
    }

    pub fn set_expiry(&mut self, expiry: utils::DateTime) {
        self.expires_at.insert(expiry.timestamp() as usize);
    }

    pub fn set_access_key(&mut self, access_key: String) {
        self.subject = access_key.clone();
        self.access_key = access_key;
    }

    pub fn validate(&self) -> Result<(), JwtError> {
        let mut verr = Vec::new();
        let now = utils::now().timestamp() as usize;
        if !verify_exp(&self.expires_at, now, false) {
            verr.push(JwtError::Expired);
        }
        if !verify_iat_or_nbf(&self.issued_at, now, false) {
            verr.push(JwtError::IssuedAt);
        }
        if !verify_iat_or_nbf(&self.not_before, now, false) {
            verr.push(JwtError::NotValidYet);
        }
        if !verr.is_empty() {
            return Err(JwtError::Aggregated(verr));
        }

        if self.access_key.is_empty() && self.subject.is_empty() {
            return Err(JwtError::Other("accessKey/subject missing".into()));
        }

        Ok(())
    }
}

impl MapClaims {
    pub fn lookup(&self, key: &str) -> &serde_json::value::Value {
        &self.0[key]
    }
}

fn verify_aud_or_iss(source: &Option<String>, target: &str, required: bool) -> bool {
    match source {
        Some(s) => constant_time_eq::constant_time_eq(s.as_bytes(), target.as_bytes()),
        None => !required,
    }
}

fn verify_exp(source: &Option<usize>, now: usize, required: bool) -> bool {
    match source {
        Some(e) => *e >= now,
        None => !required,
    }
}

fn verify_iat_or_nbf(source: &Option<usize>, now: usize, required: bool) -> bool {
    match source {
        Some(e) => *e <= now,
        None => !required,
    }
}
