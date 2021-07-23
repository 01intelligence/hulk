use std::fmt;

use anyhow::ensure;
use serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, Serializer};

pub use super::*;

// SQS resource name representation.
pub struct Arn {
    pub target_id: TargetId,
    pub(super) region: String,
}

impl fmt::Display for Arn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.target_id.id.is_empty() && self.target_id.name.is_empty() && self.region.is_empty()
        {
            return write!(f, "");
        }
        write!(f, "arn:hulk:sqs:{}:{}", self.region, self.target_id)
    }
}

impl Serialize for Arn {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Arn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PrincipalVisitor;
        impl<'de> Visitor<'de> for PrincipalVisitor {
            type Value = Arn;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an arn string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                parse_arn(v).map_err(|e| E::custom(e))
            }
        }

        deserializer.deserialize_any(PrincipalVisitor)
    }
}

fn parse_arn(s: &str) -> anyhow::Result<Arn> {
    // ARN must be in the format of arn:hulk:sqs:<REGION>:<ID>:<TYPE>
    ensure!(
        s.starts_with("arn:hulk:sqs:"),
        EventError::InvalidArn(s.to_owned())
    );
    let tokens: Vec<_> = s.split(':').collect();
    ensure!(
        tokens.len() == 6 && !tokens[4].is_empty() && !tokens[5].is_empty(),
        EventError::InvalidArn(s.to_owned())
    );
    Ok(Arn {
        target_id: TargetId {
            id: tokens[4].to_owned(),
            name: tokens[5].to_owned(),
        },
        region: tokens[3].to_owned(),
    })
}
