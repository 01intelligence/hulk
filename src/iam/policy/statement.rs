use std::collections::HashSet;
use std::fmt;
use std::hash::Hasher;

use anyhow::bail;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::*;
use crate::bucket::policy as bpolicy;
use crate::bucket::policy::{condition, Allowed, Valid};

#[derive(Clone)]
pub struct Statement<'a, 'b> {
    pub sid: bpolicy::ID,
    pub effect: bpolicy::Effect<'a>,
    pub actions: ActionSet<'b>,
    pub resources: ResourceSet,
    pub conditions: condition::Functions,
}

impl<'a, 'b> Statement<'a, 'b> {
    pub fn is_allowed(&self, args: &Args) -> bool {
        let check = || {
            if !self.actions.contains(&args.action) {
                return false;
            }
            let mut resource = args.bucket_name.clone();
            if !args.object_name.is_empty() {
                if !args.object_name.starts_with('/') {
                    resource += "/";
                }
                resource += &args.object_name;
            } else {
                resource += "/";
            }
            // For admin statements, resource match can be ignored.
            if !self.resources.is_match(&resource, &args.condition_values) && !self.is_admin() {
                return false;
            }
            return self.conditions.evaluate(&args.condition_values);
        };
        self.effect.is_allowed(check())
    }

    fn is_admin(&self) -> bool {
        self.actions.iter().any(|a| AdminAction::from(a).is_valid())
    }

    pub(super) fn is_valid(&self) -> anyhow::Result<()> {
        if !self.effect.is_valid() {
            bail!("invalid effect '{}'", self.effect);
        }
        if self.actions.is_empty() {
            bail!("empty action");
        }

        // For admin actions.
        if self.is_admin() {
            self.actions.validate_admin()?;
            for action in self.actions.iter() {
                let keys = self.conditions.keys();
                let keys_diff: condition::KeySet = keys
                    .difference(ADMIN_ACTION_CONDITION_KEY_MAP.get(&action.into()).ok_or(
                        anyhow::anyhow!("no supported condition key for action '{}'", action),
                    )?)
                    .cloned()
                    .collect();
                if !keys_diff.is_empty() {
                    bail!(
                        "unsupported condition keys '{:?}' used for action '{}'",
                        keys_diff,
                        action
                    );
                }
            }
            return Ok(());
        }

        // For non-admin actions.
        if !self.sid.is_valid() {
            bail!("invalid sid '{}'", self.sid);
        }
        if self.resources.is_empty() {
            bail!("empty resource");
        }
        self.resources.validate()?;
        self.actions.validate()?;
        for action in self.actions.iter() {
            if action.is_object_action() {
                if !self.resources.object_resource_exists() {
                    bail!(
                        "unsupported resource found '{}' for action '{}'",
                        self.resources,
                        action
                    );
                }
            } else {
                if !self.resources.bucket_resource_exists() {
                    bail!(
                        "unsupported resource found '{}' for action '{}'",
                        self.resources,
                        action
                    );
                }
            }

            let keys = self.conditions.keys();
            let keys_diff: condition::KeySet = keys
                .difference(&(action_condition_keyset(action) as HashSet<condition::Key>))
                .cloned()
                .collect();
            if !keys_diff.is_empty() {
                bail!(
                    "unsupported condition keys '{:?}' used for action '{}'",
                    keys_diff,
                    action
                );
            }
        }
        Ok(())
    }
}

impl<'a, 'b> std::cmp::PartialEq for Statement<'a, 'b> {
    fn eq(&self, other: &Self) -> bool {
        self.effect == other.effect
            && self.actions == other.actions
            && self.resources == other.resources
            && self.conditions == other.conditions
    }
}

impl<'a, 'b> std::cmp::Eq for Statement<'a, 'b> {}

impl<'a, 'b> Serialize for Statement<'a, 'b> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if self.is_valid().is_err() {
            return Err(S::Error::custom("invalid statement"));
        }
        let mut p = serializer.serialize_struct("Statement", 6)?;
        if !self.sid.is_empty() {
            p.serialize_field("Sid", &self.sid)?;
        }
        p.serialize_field("Effect", &self.effect)?;
        p.serialize_field("Action", &self.actions)?;
        p.serialize_field("Resource", &self.resources)?;
        if !self.conditions.is_empty() {
            p.serialize_field("Condition", &self.conditions)?;
        }
        p.end()
    }
}

impl<'de, 'a, 'b> Deserialize<'de> for Statement<'a, 'b> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StatementVisitor;
        impl<'de> Visitor<'de> for StatementVisitor {
            type Value = Statement<'static, 'static>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a statement")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                use serde::de::Error;
                let mut sid = None;
                let mut effect = None;
                let mut actions = None;
                let mut resources = None;
                let mut conditions = None;
                while let Ok(Some(k)) = map.next_key::<&str>() {
                    match k {
                        "Sid" => {
                            sid = Some(map.next_value()?);
                        }
                        "Effect" => {
                            effect = Some(map.next_value()?);
                        }
                        "Action" => {
                            actions = Some(map.next_value()?);
                        }
                        "Resource" => {
                            resources = Some(map.next_value()?);
                        }
                        "Condition" => {
                            conditions = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(A::Error::custom(format!(
                                "invalid statement field '{}'",
                                k
                            )));
                        }
                    }
                }
                Ok(Statement {
                    sid: sid.unwrap_or("".to_owned()),
                    effect: effect.ok_or_else(|| A::Error::missing_field("Effect"))?,
                    actions: actions.ok_or_else(|| A::Error::missing_field("Action"))?,
                    resources: resources.ok_or_else(|| A::Error::missing_field("Resource"))?,
                    conditions: conditions.unwrap_or(condition::Functions::default()),
                })
            }
        }

        deserializer.deserialize_map(StatementVisitor)
    }
}
