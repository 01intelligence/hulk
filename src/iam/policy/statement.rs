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
            if !self.resources.object_resource_exists() && !self.resources.bucket_resource_exists()
            {
                bail!(
                    "unsupported resource found '{}' for action '{}'",
                    self.resources,
                    action
                );
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
                    actions: actions.unwrap_or(ActionSet(HashSet::new())),
                    resources: resources.unwrap_or(ResourceSet::new(vec![])),
                    conditions: conditions.unwrap_or(condition::Functions::default()),
                })
            }
        }

        deserializer.deserialize_map(StatementVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bucket::policy::{condition, Effect, ALLOW, DENY};
    use crate::iam_actionset;
    use crate::prelude::HashMap;
    use crate::utils::assert::*;

    #[test]
    fn test_statement_is_allowed() -> anyhow::Result<()> {
        let statement1 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            actions: iam_actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
            conditions: Default::default(),
        };

        let statement2 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: Default::default(),
        };

        let func1 = condition::new_ip_address_func(
            condition::AWS_SOURCE_IP,
            condition::ValueSet::new(vec![condition::Value::String("192.168.1.0/24".to_string())]),
        )?;

        let statement3 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func1.clone()]),
        };

        let statement4 = Statement {
            sid: "".to_string(),
            effect: DENY,
            actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func1]),
        };

        let anon_get_bucket_location_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: GET_BUCKET_LOCATION_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: Default::default(),
            is_owner: false,
            object_name: "".to_string(),
            claims: Default::default(),
            deny_only: false,
        };

        let anon_put_object_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: PUT_OBJECT_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: HashMap::from([
                (
                    "x-amz-copy-source".to_string(),
                    vec!["mybucket/myobject".to_string()],
                ),
                ("SourceIp".to_string(), vec!["192.168.1.10".to_string()]),
            ]),
            is_owner: false,
            object_name: "myobject".to_string(),
            claims: Default::default(),
            deny_only: false,
        };

        let anon_get_object_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: GET_OBJECT_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: Default::default(),
            is_owner: false,
            object_name: "myobject".to_string(),
            claims: Default::default(),
            deny_only: false,
        };

        let get_bucket_location_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: GET_BUCKET_LOCATION_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: Default::default(),
            is_owner: false,
            object_name: "".to_string(),
            claims: Default::default(),
            deny_only: false,
        };

        let put_object_action_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: PUT_OBJECT_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: HashMap::from([
                (
                    "x-amz-copy-source".to_string(),
                    vec!["mybucket/myobject".to_string()],
                ),
                ("SourceIp".to_string(), vec!["192.168.1.10".to_string()]),
            ]),
            is_owner: false,
            object_name: "myobject".to_string(),
            claims: Default::default(),
            deny_only: false,
        };

        let get_object_action_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: GET_OBJECT_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: Default::default(),
            is_owner: false,
            object_name: "myobject".to_string(),
            claims: Default::default(),
            deny_only: false,
        };

        let cases = [
            (&statement1, &anon_get_bucket_location_args, true),
            (&statement1, &anon_put_object_args, true),
            (&statement1, &anon_get_object_args, false),
            (&statement1, &get_bucket_location_args, true),
            (&statement1, &put_object_action_args, true),
            (&statement1, &get_object_action_args, false),
            (&statement2, &anon_get_bucket_location_args, false),
            (&statement2, &anon_put_object_args, true),
            (&statement2, &anon_get_object_args, true),
            (&statement2, &get_bucket_location_args, false),
            (&statement2, &put_object_action_args, true),
            (&statement2, &get_object_action_args, true),
            (&statement3, &anon_get_bucket_location_args, false),
            (&statement3, &anon_put_object_args, true),
            (&statement3, &anon_get_object_args, false),
            (&statement3, &get_bucket_location_args, false),
            (&statement3, &put_object_action_args, true),
            (&statement3, &get_object_action_args, false),
            (&statement4, &anon_get_bucket_location_args, true),
            (&statement4, &anon_put_object_args, false),
            (&statement4, &anon_get_object_args, true),
            (&statement4, &get_bucket_location_args, true),
            (&statement4, &put_object_action_args, false),
            (&statement4, &get_object_action_args, true),
        ];

        for (statement, args, expected_result) in cases {
            let result = statement.is_allowed(args);

            assert_eq!(result, expected_result);
        }

        Ok(())
    }

    #[test]
    fn test_statement_is_valid() -> anyhow::Result<()> {
        let func1 = condition::new_ip_address_func(
            condition::AWS_SOURCE_IP,
            condition::ValueSet::new(vec![condition::Value::String("192.168.1.0/24".to_string())]),
        )?;

        let func2 = condition::new_string_equals_func(
            condition::S3X_AMZ_COPY_SOURCE,
            condition::ValueSet::new(vec![condition::Value::String(
                "mybucket/myobject".to_string(),
            )]),
        )?;

        let func3 = condition::new_string_equals_func(
            condition::AWS_USER_AGENT,
            condition::ValueSet::new(vec![condition::Value::String("NSPlayer".to_string())]),
        )?;

        let cases = [
            // Invalid effect error.
            (
                Statement {
                    sid: "".to_string(),
                    effect: Effect("foo"),
                    actions: iam_actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "*".to_string(),
                        "".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                true,
            ),
            // Empty actions error.
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(),
                    resources: ResourceSet::new(vec![Resource::new(
                        "*".to_string(),
                        "".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                true,
            ),
            // Empty resources error.
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![]),
                    conditions: Default::default(),
                },
                true,
            ),
            // Unsupported conditions for GetObject
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1.clone(), func2.clone()]),
                },
                true,
            ),
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                false,
            ),
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                false,
            ),
            (
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    actions: iam_actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1]),
                },
                false,
            ),
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(
                        Action::from(CREATE_USER_ADMIN_ACTION),
                        Action::from(DELETE_USER_ADMIN_ACTION)
                    ),
                    resources: ResourceSet::new(vec![]),
                    conditions: condition::Functions::new(vec![func2, func3]),
                },
                true,
            ),
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(
                        Action::from(CREATE_USER_ADMIN_ACTION),
                        Action::from(DELETE_USER_ADMIN_ACTION)
                    ),
                    resources: ResourceSet::new(vec![]),
                    conditions: Default::default(),
                },
                false,
            ),
        ];

        for (statement, expect_err) in cases {
            if !expect_err {
                assert_ok!(statement.is_valid());
            } else {
                assert_err!(statement.is_valid());
            }
        }

        Ok(())
    }

    #[test]
    fn test_statement_deserialize_json() -> anyhow::Result<()> {
        let data1 = r#"{
            "Sid": "SomeId1",
            "Effect": "Allow",
            "Action": "s3:PutObject",
            "Resource": "arn:aws:s3:::mybucket/myobject*"
        }"#;

        let statement1 = Statement {
            sid: "SomeId1".to_string(),
            effect: ALLOW,
            actions: iam_actionset!(PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: Default::default(),
        };

        let data2 = r#"{
            "Effect": "Allow",
            "Action": "s3:PutObject",
            "Resource": "arn:aws:s3:::mybucket/myobject*",
            "Condition": {
                "Null": {
                    "s3:x-amz-copy-source": true
                }
            }
        }"#;

        let func1 = condition::new_null_func(
            condition::S3X_AMZ_COPY_SOURCE,
            condition::ValueSet::new(vec![condition::Value::Bool(true)]),
        )?;

        let statement2 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            actions: iam_actionset!(PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func1]),
        };

        let data3 = r#"{
            "Effect": "Deny",
            "Action": [
                "s3:PutObject",
                "s3:GetObject"
            ],
            "Resource": "arn:aws:s3:::mybucket/myobject*",
            "Condition": {
                "Null": {
                    "s3:x-amz-server-side-encryption": "false"
                }
            }
        }"#;

        let func2 = condition::new_null_func(
            condition::S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            condition::ValueSet::new(vec![condition::Value::Bool(false)]),
        )?;

        let statement3 = Statement {
            sid: "".to_string(),
            effect: DENY,
            actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func2]),
        };

        let data4 = r#"{
            "Effect": "Allow",
            "Action": "s3:PutObjec,
            "Resource": "arn:aws:s3:::mybucket/myobject*"
        }"#;

        let data5 = r#"{
            "Action": "s3:PutObject",
            "Resource": "arn:aws:s3:::mybucket/myobject*"
        }"#;

        let data6 = r#"{
            "Effect": "Allow",
            "Resource": "arn:aws:s3:::mybucket/myobject*"
        }"#;

        let data7 = r#"{
            "Effect": "Allow",
            "Action": "s3:PutObject"
        }"#;

        let data8 = r#"{
            "Effect": "Allow",
            "Action": "s3:PutObject",
            "Resource": "arn:aws:s3:::mybucket/myobject*",
            "Condition": {
            }
        }"#;

        let data9 = r#"{
            "Effect": "Deny",
            "Action": [
                "s3:PutObject",
                "s3:GetObject"
            ],
            "Resource": "arn:aws:s3:::mybucket/myobject*",
            "Condition": {
                "StringEquals": {
                    "s3:x-amz-copy-source": "yourbucket/myobject*"
                }
            }
        }"#;

        let cases = [
            (data1, Some(statement1), false, false),
            (data2, Some(statement2), false, false),
            (data3, Some(statement3), false, false),
            // JSON deserialize error.
            (data4, None, true, false),
            // Invalid effect error.
            (data5, None, true, true), // TODO: default effect
            // Empty action error.
            (data6, None, false, true),
            // Empty resource error.
            (data7, None, false, true),
            // Empty condition error.
            (data8, None, false, false), // TODO: default actions
            // Unsupported condition key error.
            (data9, None, false, true),
        ];

        for (data, expected_result, expect_deserialize_err, expect_validation_err) in cases {
            let result = serde_json::from_str::<Statement>(data);
            println!("111111111");

            match result {
                Ok(result) => {
                    if !expect_validation_err {
                        assert_ok!(result.is_valid());
                    } else {
                        assert_err!(result.is_valid());
                    }
                }
                Err(err) => assert!(expect_deserialize_err, "expect an error"),
            }
        }

        Ok(())
    }

    #[test]
    fn test_statement_validate() -> anyhow::Result<()> {
        let statement1 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            actions: iam_actionset!(PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "myobject".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: Default::default(),
        };

        let func1 = condition::new_null_func(
            condition::S3X_AMZ_COPY_SOURCE,
            condition::ValueSet::new(vec![condition::Value::Bool(true)]),
        )?;

        let func2 = condition::new_null_func(
            condition::S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            condition::ValueSet::new(vec![condition::Value::Bool(false)]),
        )?;

        let statement2 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "myobject".to_string(),
                "myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func1, func2]),
        };

        let cases = [(statement1, false), (statement2, true)];

        for (statement, expect_err) in cases {
            if !expect_err {
                assert_ok!(statement.is_valid());
            } else {
                assert_err!(statement.is_valid());
            }
        }

        Ok(())
    }
}
