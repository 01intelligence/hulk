use std::fmt;
use std::hash::Hasher;

use anyhow::bail;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::*;

#[derive(Clone)]
pub struct Statement<'a, 'b> {
    pub sid: ID,
    pub effect: Effect<'a>,
    pub principal: Principal,
    pub actions: ActionSet<'b>,
    pub resources: ResourceSet,
    pub conditions: condition::Functions,
}

impl<'a, 'b> Statement<'a, 'b> {
    pub fn is_allowed(&self, args: &Args) -> bool {
        let check = || {
            if !self.principal.is_match(&args.account_name) {
                return false;
            }
            if !self.actions.contains(&args.action) {
                return false;
            }
            let mut resource = args.bucket_name.clone();
            if !args.object_name.is_empty() {
                if !args.object_name.starts_with('/') {
                    resource += "/";
                }
                resource += &args.object_name;
            }
            if !self.resources.is_match(&resource, &args.condition_values) {
                return false;
            }
            return self.conditions.evaluate(&args.condition_values);
        };
        self.effect.is_allowed(check())
    }

    pub fn validate(&self, bucket_name: &str) -> anyhow::Result<()> {
        let _ = self.is_valid()?;
        self.resources.validate(bucket_name)
    }

    pub(super) fn is_valid(&self) -> anyhow::Result<()> {
        if !self.effect.is_valid() {
            bail!("invalid effect '{}'", self.effect);
        }
        if !self.principal.is_valid() {
            bail!("invalid principal '{:?}'", self.principal);
        }
        if self.actions.is_empty() {
            bail!("empty action");
        }
        if self.resources.is_empty() {
            bail!("empty resource");
        }
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
                .difference(ACTION_CONDITION_KEY_MAP.get(action).ok_or(anyhow::anyhow!(
                    "no supported condition key for action '{}'",
                    action
                ))?)
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
            && self.principal == other.principal
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
        p.serialize_field("Principal", &self.principal)?;
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
                let mut principal = None;
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
                        "Principal" => {
                            principal = Some(map.next_value()?);
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
                    principal: principal.ok_or_else(|| A::Error::missing_field("Principal"))?,
                    actions: actions.ok_or_else(|| A::Error::missing_field("Action"))?,
                    resources: resources.ok_or_else(|| A::Error::missing_field("Resource"))?,
                    conditions: conditions.unwrap_or(condition::Functions::default()),
                })
            }
        }

        deserializer.deserialize_map(StatementVisitor)
    }
}
