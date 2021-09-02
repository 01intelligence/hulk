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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::HashMap;
    use crate::utils::assert::*;
    use crate::{actionset, principal};

    #[test]
    fn test_statement_is_allowed() -> anyhow::Result<()> {
        let statement1 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            principal: principal!("*".to_string()),
            actions: actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
            conditions: condition::Functions::new(vec![]),
        };

        let statement2 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            principal: principal!("*".to_string()),
            actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![]),
        };

        let func1 = condition::new_ip_address_func(
            condition::AWS_SOURCE_IP,
            condition::ValueSet::new(vec![condition::Value::String("192.168.1.0/24".to_string())]),
        )?;

        let statement3 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            principal: principal!("*".to_string()),
            actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func1.clone()]),
        };

        let statement4 = Statement {
            sid: "".to_string(),
            effect: DENY,
            principal: principal!("*".to_string()),
            actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
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
            condition_values: HashMap::new(),
            is_owner: false,
            object_name: "".to_string(),
        };

        let anon_put_object_action_args = Args {
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
        };

        let anon_get_object_action_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: GET_OBJECT_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: HashMap::new(),
            is_owner: false,
            object_name: "myobject".to_string(),
        };

        let get_bucket_location_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: GET_BUCKET_LOCATION_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: HashMap::new(),
            is_owner: true,
            object_name: "".to_string(),
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
            is_owner: true,
            object_name: "myobject".to_string(),
        };

        let get_object_action_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: GET_OBJECT_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: HashMap::new(),
            is_owner: true,
            object_name: "myobject".to_string(),
        };

        let cases = [
            (&statement1, &anon_get_bucket_location_args, true),
            (&statement1, &anon_put_object_action_args, true),
            (&statement1, &anon_get_object_action_args, false),
            (&statement1, &get_bucket_location_args, true),
            (&statement1, &put_object_action_args, true),
            (&statement1, &get_object_action_args, false),
            (&statement2, &anon_get_bucket_location_args, false),
            (&statement2, &anon_put_object_action_args, true),
            (&statement2, &anon_get_object_action_args, true),
            (&statement2, &get_bucket_location_args, false),
            (&statement2, &put_object_action_args, true),
            (&statement2, &get_object_action_args, true),
            (&statement3, &anon_get_bucket_location_args, false),
            (&statement3, &anon_put_object_action_args, true),
            (&statement3, &anon_get_object_action_args, false),
            (&statement3, &get_bucket_location_args, false),
            (&statement3, &put_object_action_args, true),
            (&statement3, &get_object_action_args, false),
            (&statement4, &anon_get_bucket_location_args, true),
            (&statement4, &anon_put_object_action_args, false),
            (&statement4, &anon_get_object_action_args, true),
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

        let func2 = condition::new_string_not_equals_func(
            condition::S3X_AMZ_COPY_SOURCE,
            condition::ValueSet::new(vec![condition::Value::String(
                "mybucket/myobject".to_string(),
            )]),
        )?;

        let cases = [
            // Invalid effect error.
            // (
            //     Statement {
            //         sid: "".to_string(),
            //         effect: Effect("foo"),
            //         principal: principal!("*".to_string()),
            //         actions: actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
            //         resources: ResourceSet::new(vec![Resource::new(
            //             "*".to_string(),
            //             "".to_string(),
            //         )]),
            //         conditions: Default::default(),
            //     },
            //     true,
            // ),
            // Invalid principal error.
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: Default::default(),
                    actions: actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
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
                    principal: principal!("*".to_string()),
                    actions: Default::default(),
                    resources: ResourceSet::new(vec![Resource::new(
                        "*".to_string(),
                        "".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                true,
            ),
            // Empty resources error
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(GET_BUCKET_LIFECYCLE_ACTION, PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                true,
            ),
            // Unsupported resource found for bucket action.
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(GET_BUCKET_LIFECYCLE_ACTION, PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                true,
            ),
            // Unsupported condition key for action.
            (
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1.clone(), func2]),
                },
                true,
            ),
            (
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    principal: principal!("*".to_string()),
                    actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1]),
                },
                false,
            ),
        ];

        for (statement, expect_err) in cases {
            if expect_err {
                assert_err!(statement.is_valid());
            } else {
                assert_ok!(statement.is_valid());
            }
        }

        Ok(())
    }

    #[test]
    fn test_statement_serialize_json() -> anyhow::Result<()> {
        let statement1 = Statement {
            sid: "SomeId1".to_string(),
            effect: ALLOW,
            principal: principal!("*".to_string()),
            actions: actionset!(PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: Default::default(),
        };

        let data1 = r#"{"Sid":"SomeId1","Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"]}"#;

        let func1 = condition::new_null_func(
            condition::S3X_AMZ_COPY_SOURCE,
            condition::ValueSet::new(vec![condition::Value::Bool(true)]),
        )?;

        let statement2 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            principal: principal!("*".to_string()),
            actions: actionset!(PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func1.clone()]),
        };

        let data2 = r#"{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"],"Condition":{"Null":{"s3:x-amz-copy-source":[true]}}}"#;

        let func2 = condition::new_null_func(
            condition::S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            condition::ValueSet::new(vec![condition::Value::Bool(false)]),
        )?;

        let statement3 = Statement {
            sid: "".to_string(),
            effect: DENY,
            principal: principal!("*".to_string()),
            actions: actionset!(PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func2.clone()]),
        };

        let data3 = r#"{"Effect":"Deny","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"],"Condition":{"Null":{"s3:x-amz-server-side-encryption":[false]}}}"#;

        let statement4 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            principal: principal!("*".to_string()),
            actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func1, func2]),
        };

        let cases = [
            (statement1, data1, false),
            (statement2, data2, false),
            (statement3, data3, false),
            (statement4, "", true),
        ];

        for (statement, expected_result, expect_err) in cases {
            let result = serde_json::to_string(&statement);

            match result {
                Ok(result) => assert_eq!(result, expected_result),
                Err(_) => assert!(expect_err),
            }
        }

        Ok(())
    }

    #[test]
    fn test_statement_deserialize_json() -> anyhow::Result<()> {
        let data1 = r#"{
            "Sid": "SomeId1",
            "Effect": "Allow",
            "Principal": "*",
            "Action": "s3:PutObject",
            "Resource": "arn:aws:s3:::mybucket/myobject*"
        }"#;

        let statement1 = Statement {
            sid: "SomeId1".to_string(),
            effect: ALLOW,
            principal: principal!("*".to_string()),
            actions: actionset!(PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: Default::default(),
        };

        let data2 = r#"{
            "Effect": "Allow",
            "Principal": "*",
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
            principal: principal!("*".to_string()),
            actions: actionset!(PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func1]),
        };

        let data3 = r#"{
            "Sid": "",
            "Effect": "Deny",
            "Principal": {
                "AWS": "*"
            },
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
            principal: principal!("*".to_string()),
            actions: actionset!(PUT_OBJECT_ACTION, GET_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "/myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func2]),
        };

        let data4 = r#"{
            "Effect": "Allow",
            "Principal": "Q3AM3UQ867SPQQA43P2F",
            "Action": "s3:PutObject",
            "Resource": "arn:aws:s3:::mybucket/myobject*"
        }"#;

        let data5 = r#"{
            "Principal": "*",
            "Action": "s3:PutObject",
            "Resource": "arn:aws:s3:::mybucket/myobject*"
        }"#;

        let data6 = r#"{
            "Effect": "Allow",
            "Action": "s3:PutObject",
            "Resource": "arn:aws:s3:::mybucket/myobject*"
        }"#;

        let data7 = r#"{
            "Effect": "Allow",
            "Principal": "*",
            "Resource": "arn:aws:s3:::mybucket/myobject*"
        }"#;

        let data8 = r#"{
            "Effect": "Allow",
            "Principal": "*",
            "Action": "s3:PutObject"
        }"#;

        let data9 = r#"{
            "Effect": "Allow",
            "Principal": "*",
            "Action": "s3:PutObject",
            "Resource": "arn:aws:s3:::mybucket/myobject*",
            "Condition": {
            }
        }"#;

        let data10 = r#"{
            "Effect": "Deny",
            "Principal": {
                "AWS": "*"
            },
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
            (data1, Some(statement1), false),
            (data2, Some(statement2), false),
            (data3, Some(statement3), false),
            // JSON deserializing error.
            (data4, None, true),
            // Invalid effect error.
            (data5, None, true),
            // Empty principal error.
            (data6, None, true),
            // Empty action error.
            (data7, None, true),
            // Empty resource error.
            (data8, None, true),
            // Empty condition error.
            (data9, None, true),
            // Unsupported condition key error.
            (data10, None, true),
        ];

        for (data, expected_result, expect_err) in cases {
            let result = serde_json::from_str::<Statement>(data);

            match result {
                Ok(result) => {
                    if let Some(expected_result) = expected_result {
                        println!("{}", result == expected_result);
                        assert!(result == expected_result);
                    }
                }
                Err(err) => assert!(expect_err),
            }
        }

        Ok(())
    }

    #[test]
    fn test_statement_validate() -> anyhow::Result<()> {
        let statement1 = Statement {
            sid: "".to_string(),
            effect: ALLOW,
            principal: principal!("*".to_string()),
            actions: actionset!(PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
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
            principal: principal!("*".to_string()),
            actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
            resources: ResourceSet::new(vec![Resource::new(
                "mybucket".to_string(),
                "myobject*".to_string(),
            )]),
            conditions: condition::Functions::new(vec![func1, func2]),
        };

        let cases = [
            (&statement1, "mybucket", false),
            (&statement2, "mybucket", true),
            (&statement1, "yourbucket", true),
        ];

        for (statement, bucket_name, expect_err) in cases {
            if expect_err {
                assert_err!(statement.validate(bucket_name));
            } else {
                assert_ok!(statement.validate(bucket_name));
            }
        }

        Ok(())
    }
}
