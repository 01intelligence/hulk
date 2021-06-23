use std::collections::{HashMap, HashSet};
use std::fmt;

use anyhow::bail;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::*;

// Default policy version as per AWS S3 specification.
pub const DEFAULT_VERSION: &str = "2012-10-17";

// Arguments to policy to check whether it is allowed
pub struct Args<'a> {
    pub account_name: String,
    pub groups: Vec<String>,
    pub action: Action<'a>,
    pub bucket_name: String,
    pub condition_values: HashMap<String, Vec<String>>,
    pub is_owner: bool,
    pub object_name: String,
}

// Bucket policy.
pub struct Policy<'a, 'b> {
    pub id: ID,
    pub version: String,
    pub statements: Vec<Statement<'a, 'b>>,
}

impl<'a, 'b> Policy<'a, 'b> {
    // Checks given policy args is allowed to continue the Rest API.
    pub fn is_allowed(&self, args: &Args) -> bool {
        // Check all deny statements. If any one statement denies, return false.
        for statement in &self.statements {
            if statement.effect == DENY && !statement.is_allowed(args) {
                return false;
            }
        }
        // For owner, its allowed by default.
        if args.is_owner {
            return true;
        }
        // Check all allow statements. If any one statement allows, return true.
        for statement in &self.statements {
            if statement.effect == ALLOW && statement.is_allowed(args) {
                return true;
            }
        }
        false
    }

    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    // Validates all statements are for given bucket or not.
    pub fn validate(&self, bucket_name: &str) -> anyhow::Result<()> {
        let _ = self.is_valid()?;
        for statement in &self.statements {
            let _ = statement.validate(bucket_name)?;
        }
        Ok(())
    }

    fn is_valid(&self) -> anyhow::Result<()> {
        if !self.version.is_empty() && self.version != DEFAULT_VERSION {
            bail!("invalid version '{}'", self.version);
        }
        for statement in &self.statements {
            let _ = statement.is_valid()?;
        }
        Ok(())
    }

    // Merges two policies documents and drop
    // duplicate statements if any.
    pub fn merge(&self, input: &Policy<'a, 'b>) -> Policy {
        let mut merged = Policy {
            id: "".to_string(),
            version: if !self.version.is_empty() {
                self.version.clone()
            } else {
                input.version.clone()
            },
            statements: self
                .statements
                .iter()
                .chain(input.statements.iter())
                .cloned()
                .collect(),
        };
        merged.drop_duplicate_statements();
        merged
    }

    fn drop_duplicate_statements(&mut self) {
        let mut remove_index = usize::MAX;
        'outer: for i in 0..self.statements.len() {
            for (j, statement) in self.statements[i + 1..].iter().enumerate() {
                if &self.statements[i] != statement {
                    continue;
                }
                remove_index = j;
                break 'outer;
            }
        }
        if remove_index < usize::MAX {
            self.statements.remove(remove_index);
            self.drop_duplicate_statements();
        }
    }
}

impl<'a, 'b> Serialize for Policy<'a, 'b> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if self.is_valid().is_err() {
            return Err(S::Error::custom("invalid policy"));
        }
        let mut p = serializer.serialize_struct("Policy", 3)?;
        p.serialize_field("ID", &self.id)?;
        p.serialize_field("Version", &self.version)?;
        p.serialize_field("Statement", &self.statements)?;
        p.end()
    }
}

impl<'de, 'a, 'b> Deserialize<'de> for Policy<'a, 'b> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PolicyVisitor;
        impl<'de> Visitor<'de> for PolicyVisitor {
            type Value = Policy<'static, 'static>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a policy")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                use serde::de::Error;
                let mut id = None;
                let mut version = None;
                let mut statements = None;
                while let Ok(Some(k)) = map.next_key::<&str>() {
                    match k {
                        "ID" => {
                            id = Some(map.next_value()?);
                        }
                        "Version" => {
                            version = Some(map.next_value()?);
                        }
                        "Statement" => {
                            statements = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(A::Error::custom(format!("invalid policy field '{}'", k)));
                        }
                    }
                }
                let mut policy = Policy {
                    id: id.ok_or_else(|| A::Error::missing_field("ID"))?,
                    version: version.ok_or_else(|| A::Error::missing_field("Version"))?,
                    statements: statements.ok_or_else(|| A::Error::missing_field("Statement"))?,
                };
                if let Err(e) = policy.is_valid() {
                    return Err(A::Error::custom(format!("invalid policy: {}", e)));
                }
                policy.drop_duplicate_statements();
                Ok(policy)
            }
        }

        deserializer.deserialize_map(PolicyVisitor)
    }
}
