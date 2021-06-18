use std::fmt;

use lazy_static::lazy_static;
use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::bucket::policy::Valid;

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct Name<'a>(&'a str);

pub(super) const STRING_EQUALS: Name = Name("StringEquals");
pub(super) const STRING_NOT_EQUALS: Name = Name("StringNotEquals");
pub(super) const STRING_EQUALS_IGNORE_CASE: Name = Name("StringEqualsIgnoreCase");
pub(super) const STRING_NOT_EQUALS_IGNORE_CASE: Name = Name("StringNotEqualsIgnoreCase");
pub(super) const STRING_LIKE: Name = Name("StringLike");
pub(super) const STRING_NOT_LIKE: Name = Name("StringNotLike");
pub(super) const BINARY_EQUALS: Name = Name("BinaryEquals");
pub(super) const IP_ADDRESS: Name = Name("IpAddress");
pub(super) const NOT_IP_ADDRESS: Name = Name("NotIpAddress");
pub(super) const NULL: Name = Name("Null");
pub(super) const BOOLEAN: Name = Name("Bool");
pub(super) const NUMERIC_EQUALS: Name = Name("NumericEquals");
pub(super) const NUMERIC_NOT_EQUALS: Name = Name("NumericNotEquals");
pub(super) const NUMERIC_LESS_THAN: Name = Name("NumericLessThan");
pub(super) const NUMERIC_LESS_THAN_EQUALS: Name = Name("NumericLessThanEquals");
pub(super) const NUMERIC_GREATER_THAN: Name = Name("NumericGreaterThan");
pub(super) const NUMERIC_GREATER_THAN_EQUALS: Name = Name("NumericGreaterThanEquals");
pub(super) const DATE_EQUALS: Name = Name("DateEquals");
pub(super) const DATE_NOT_EQUALS: Name = Name("DateNotEquals");
pub(super) const DATE_LESS_THAN: Name = Name("DateLessThan");
pub(super) const DATE_LESS_THAN_EQUALS: Name = Name("DateLessThanEquals");
pub(super) const DATE_GREATER_THAN: Name = Name("DateGreaterThan");
pub(super) const DATE_GREATER_THAN_EQUALS: Name = Name("DateGreaterThanEquals");

lazy_static! {
    pub(super) static ref SUPPORTED_CONDITIONS: Vec<Name<'static>> = vec![
        STRING_EQUALS,
        STRING_NOT_EQUALS,
        STRING_EQUALS_IGNORE_CASE,
        STRING_NOT_EQUALS_IGNORE_CASE,
        STRING_LIKE,
        STRING_NOT_LIKE,
        BINARY_EQUALS,
        IP_ADDRESS,
        NOT_IP_ADDRESS,
        NULL,
        BOOLEAN,
        NUMERIC_EQUALS,
        NUMERIC_NOT_EQUALS,
        NUMERIC_LESS_THAN,
        NUMERIC_LESS_THAN_EQUALS,
        NUMERIC_GREATER_THAN,
        NUMERIC_GREATER_THAN_EQUALS,
        DATE_EQUALS,
        DATE_NOT_EQUALS,
        DATE_LESS_THAN,
        DATE_LESS_THAN_EQUALS,
        DATE_GREATER_THAN,
        DATE_GREATER_THAN_EQUALS,
    ];
}

impl<'a> Valid for Name<'a> {
    fn is_valid(&self) -> bool {
        SUPPORTED_CONDITIONS.iter().any(|n| self == n)
    }
}

impl<'a> fmt::Display for Name<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> Serialize for Name<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if !self.is_valid() {
            return Err(S::Error::custom(format!("unknown name '{}'", self.0)));
        }
        serializer.serialize_newtype_struct("Name", &self.0)
    }
}

impl<'de, 'a> Deserialize<'de> for Name<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NameVisitor;
        impl<'de> Visitor<'de> for NameVisitor {
            type Value = Name<'static>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a condition name string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                SUPPORTED_CONDITIONS
                    .iter()
                    .find(|&k| k.0 == v)
                    .cloned()
                    .ok_or(E::custom(format!("invalid condition name '{}'", v)))
            }
        }

        deserializer.deserialize_str(NameVisitor)
    }
}
