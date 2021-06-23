use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::Valid;

// Policy statement effect Allow or Deny.
#[derive(Eq, PartialEq, Clone)]
pub struct Effect<'a>(&'a str);

pub const ALLOW: Effect = Effect("Allow"); // allow effect
pub const DENY: Effect = Effect("Deny"); // deny effect

impl<'a> super::Allowed for Effect<'a> {
    fn is_allowed(&self, b: bool) -> bool {
        if *self == ALLOW {
            b
        } else {
            !b
        }
    }
}

impl<'a> Valid for Effect<'a> {
    fn is_valid(&self) -> bool {
        match *self {
            ALLOW | DENY => true,
            _ => false,
        }
    }
}

impl<'a> fmt::Display for Effect<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> Serialize for Effect<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if !self.is_valid() {
            return Err(S::Error::custom(format!("invalid effect '{}'", self.0)));
        }
        serializer.serialize_str(self.0)
    }
}

impl<'de, 'a> Deserialize<'de> for Effect<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EffectVisitor;
        impl<'de> Visitor<'de> for EffectVisitor {
            type Value = Effect<'static>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an effect")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match Effect(v) {
                    ALLOW => Ok(ALLOW),
                    DENY => Ok(DENY),
                    _ => Err(E::custom(format!("invalid effect '{}'", v))),
                }
            }
        }

        deserializer.deserialize_str(EffectVisitor)
    }
}
