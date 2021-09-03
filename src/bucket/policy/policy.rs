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

impl<'a, 'b> PartialEq for Policy<'a, 'b> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.version == other.version && self.statements == other.statements
    }
}

impl<'a, 'b> Eq for Policy<'a, 'b> {}

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
        if !self.id.is_empty() {
            p.serialize_field("ID", &self.id)?;
        }
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
                    id: id.unwrap_or("".to_owned()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::assert::*;
    use crate::{actionset, principal};

    #[test]
    fn test_policy_is_allowed() -> anyhow::Result<()> {
        let policy1 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
                conditions: Default::default(),
            }],
        };

        let policy2 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: Default::default(),
            }],
        };

        let func1 = condition::new_ip_address_func(
            condition::AWS_SOURCE_IP,
            condition::ValueSet::new(vec![condition::Value::String("192.168.1.0/24".to_string())]),
        )?;

        let policy3 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: condition::Functions::new(vec![func1.clone()]),
            }],
        };

        let policy4 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: DENY,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: condition::Functions::new(vec![func1]),
            }],
        };

        let anon_get_bucket_location_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: GET_BUCKET_LOCATION_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: Default::default(),
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
            condition_values: Default::default(),
            is_owner: false,
            object_name: "myobject".to_string(),
        };

        let get_bucket_location_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: GET_BUCKET_LOCATION_ACTION,
            bucket_name: "mybucket".to_string(),
            condition_values: Default::default(),
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
            condition_values: Default::default(),
            is_owner: true,
            object_name: "myobject".to_string(),
        };

        let cases = [
            (&policy1, &anon_get_bucket_location_args, true),
            (&policy1, &anon_put_object_action_args, true),
            (&policy1, &anon_get_object_action_args, false),
            (&policy1, &get_bucket_location_args, true),
            (&policy1, &put_object_action_args, true),
            (&policy1, &get_object_action_args, true),
            (&policy2, &anon_get_bucket_location_args, false),
            (&policy2, &anon_put_object_action_args, true),
            (&policy2, &anon_get_object_action_args, true),
            (&policy2, &get_bucket_location_args, true),
            (&policy2, &put_object_action_args, true),
            (&policy2, &get_object_action_args, true),
            (&policy3, &anon_get_bucket_location_args, false),
            (&policy3, &anon_put_object_action_args, true),
            (&policy3, &anon_get_object_action_args, false),
            (&policy3, &get_bucket_location_args, true),
            (&policy3, &put_object_action_args, true),
            (&policy3, &get_object_action_args, true),
            (&policy4, &anon_get_bucket_location_args, false),
            (&policy4, &anon_put_object_action_args, false),
            (&policy4, &anon_get_object_action_args, false),
            (&policy4, &get_bucket_location_args, true),
            (&policy4, &put_object_action_args, false),
            (&policy4, &get_object_action_args, true),
        ];

        for (policy, args, expected_result) in cases {
            let result = policy.is_allowed(args);
            println!("-----");

            assert_eq!(result, expected_result);
        }

        Ok(())
    }

    #[test]
    fn test_policy_is_empty() {
        let policy1 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: Default::default(),
            }],
        };

        let policy2 = Policy {
            id: "MyPolicyForMyBucket".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![],
        };

        let cases = [(policy1, false), (policy2, true)];

        for (policy, expected_result) in cases {
            let result = policy.is_empty();

            assert_eq!(result, expected_result);
        }
    }

    #[test]
    fn test_policy_is_valid() -> anyhow::Result<()> {
        let policy1 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: Default::default(),
            }],
        };

        let policy2 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    principal: principal!("*".to_string()),
                    actions: actionset!(GET_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let policy3 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/yourobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let func1 = condition::new_null_func(
            condition::S3X_AMZ_COPY_SOURCE,
            condition::ValueSet::new(vec![condition::Value::Bool(true)]),
        )?;

        let func2 = condition::new_null_func(
            condition::S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            condition::ValueSet::new(vec![condition::Value::Bool(false)]),
        )?;

        let policy4 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1.clone()]),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/yourobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func2.clone()]),
                },
            ],
        };

        let policy5 = Policy {
            id: "".to_string(),
            version: "17-10-2012".to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: Default::default(),
            }],
        };

        let policy6 = Policy {
            id: "MyPolicyForMyBucket".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "myobject*".to_string(),
                )]),
                conditions: condition::Functions::new(vec![func1, func2]),
            }],
        };

        let policy7 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let policy8 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let cases = [
            (policy1, false),
            // Allowed duplicate principal.
            (policy2, false),
            // Allowed duplicate principal and action.
            (policy3, false),
            // Allowed duplicate principal, action and resource.
            (policy4, false),
            // Invalid version error.
            (policy5, true),
            // Invalid statement error.
            (policy6, true),
            // Duplicate statement success different effects.
            (policy7, false),
            // Duplicate statement success, duplicate statement dropped.
            (policy8, false),
        ];

        for (policy, expect_err) in cases {
            if expect_err {
                assert_err!(policy.is_valid());
            } else {
                assert_ok!(policy.is_valid())
            }
        }

        Ok(())
    }

    #[test]
    fn test_policy_serialize_json() -> anyhow::Result<()> {
        let policy1 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "SomeId1".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: Default::default(),
            }],
        };

        let data1 = r#"{"ID":"MyPolicyForMyBucket1","Version":"2012-10-17","Statement":[{"Sid":"SomeId1","Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"]}]}"#;

        let func1 = condition::new_ip_address_func(
            condition::AWS_SOURCE_IP,
            condition::ValueSet::new(vec![condition::Value::String("192.168.1.0/24".to_string())]),
        )?;

        let policy2 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    principal: principal!("*".to_string()),
                    actions: actionset!(GET_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/yourobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1.clone()]),
                },
            ],
        };

        let data2 = r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"]},{"Effect":"Deny","Principal":{"AWS":["*"]},"Action":["s3:GetObject"],"Resource":["arn:aws:s3:::mybucket/yourobject*"],"Condition":{"IpAddress":{"aws:SourceIp":["192.168.1.0/24"]}}}]}"#;

        let policy3 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("Q3AM3UQ867SPQQA43P2F".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let data3 = r#"{"ID":"MyPolicyForMyBucket1","Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"AWS":["Q3AM3UQ867SPQQA43P2F"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"]},{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"]}]}"#;

        let policy4 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(GET_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let data4 = r#"{"ID":"MyPolicyForMyBucket1","Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"]},{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:GetObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"]}]}"#;

        let policy5 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/yourobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let data5 = r#"{"ID":"MyPolicyForMyBucket1","Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"]},{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/yourobject*"]}]}"#;

        let func2 = condition::new_ip_address_func(
            condition::AWS_SOURCE_IP,
            condition::ValueSet::new(vec![condition::Value::String("192.168.2.0/24".to_string())]),
        )?;

        let policy6 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1.clone()]),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func2.clone()]),
                },
            ],
        };

        let data6 = r#"{"ID":"MyPolicyForMyBucket1","Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"],"Condition":{"IpAddress":{"aws:SourceIp":["192.168.1.0/24"]}}},{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:PutObject"],"Resource":["arn:aws:s3:::mybucket/myobject*"],"Condition":{"IpAddress":{"aws:SourceIp":["192.168.2.0/24"]}}}]}"#;

        let policy7 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_BUCKET_LOCATION_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "".to_string(),
                )]),
                conditions: Default::default(),
            }],
        };

        let data7 = r#"{"ID":"MyPolicyForMyBucket1","Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:GetBucketLocation"],"Resource":["arn:aws:s3:::mybucket"]}]}"#;

        let policy8 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_BUCKET_LOCATION_ACTION),
                resources: ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
                conditions: Default::default(),
            }],
        };

        let data8 = r#"{"ID":"MyPolicyForMyBucket1","Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:GetBucketLocation"],"Resource":["arn:aws:s3:::*"]}]}"#;

        let func3 = condition::new_null_func(
            condition::S3X_AMZ_COPY_SOURCE,
            condition::ValueSet::new(vec![condition::Value::Bool(true)]),
        )?;

        let policy9 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "myobject*".to_string(),
                )]),
                conditions: condition::Functions::new(vec![func1, func2, func3]),
            }],
        };

        let cases = [
            (policy1, data1, false),
            (policy2, data2, false),
            (policy3, data3, false),
            (policy4, data4, false),
            (policy5, data5, false),
            (policy6, data6, false),
            (policy7, data7, false),
            (policy8, data8, false),
            (policy9, "", true),
        ];

        for (policy, expected_result, expect_err) in cases {
            let result = serde_json::to_string(&policy);
            println!("----");

            match result {
                Ok(result) => assert_eq!(result, expected_result),
                Err(_) => assert!(expect_err),
            }
        }

        Ok(())
    }

    #[test]
    fn test_policy_deserialize_json() -> anyhow::Result<()> {
        let data1 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Sid": "SomeId1",
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                }
            ]
        }"#;

        let policy1 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "SomeId1".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: Default::default(),
            }],
        };

        let data2 = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Deny",
                    "Principal": "*",
                    "Action": "s3:GetObject",
                    "Resource": "arn:aws:s3:::mybucket/yourobject*",
                    "Condition": {
                        "IpAddress": {
                            "aws:SourceIp": "192.168.1.0/24"
                        }
                    }
                }
            ]
        }"#;

        let func1 = condition::new_ip_address_func(
            condition::AWS_SOURCE_IP,
            condition::ValueSet::new(vec![condition::Value::String("192.168.1.0/24".to_string())]),
        )?;

        let policy2 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    principal: principal!("*".to_string()),
                    actions: actionset!(GET_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/yourobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1.clone()]),
                },
            ],
        };

        let data3 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": {
                        "AWS": [
                            "Q3AM3UQ867SPQQA43P2F"
                        ]
                    },
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                }
            ]
        }"#;

        let policy3 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("Q3AM3UQ867SPQQA43P2F".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let data4 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:GetObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                }
            ]
        }"#;

        let policy4 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(GET_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let data5 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/yourobject*"
                }
            ]
        }"#;

        let policy5 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/yourobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let data6 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*",
                    "Condition": {
                        "IpAddress": {
                            "aws:SourceIp": "192.168.1.0/24"
                        }
                    }
                },
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*",
                    "Condition": {
                        "IpAddress": {
                            "aws:SourceIp": "192.168.2.0/24"
                        }
                    }
                }
            ]
        }"#;

        let func2 = condition::new_ip_address_func(
            condition::AWS_SOURCE_IP,
            condition::ValueSet::new(vec![condition::Value::String("192.168.2.0/24".to_string())]),
        )?;

        let policy6 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1.clone()]),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func2.clone()]),
                },
            ],
        };

        let data7 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:GetBucketLocation",
                    "Resource": "arn:aws:s3:::mybucket"
                }
            ]
        }"#;

        let policy7 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_BUCKET_LOCATION_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "".to_string(),
                )]),
                conditions: Default::default(),
            }],
        };

        let data8 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:GetBucketLocation",
                    "Resource": "arn:aws:s3:::*"
                }
            ]
        }"#;

        let policy8 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_BUCKET_LOCATION_ACTION),
                resources: ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
                conditions: Default::default(),
            }],
        };

        let data9 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "17-10-2012",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                }
            ]
        }"#;

        let data10 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                }
            ]
        }"#;

        let policy10 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "myobject*".to_string(),
                )]),
                conditions: Default::default(),
            }],
        };

        let data11 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Deny",
                    "Principal": "*",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                }
            ]
        }"#;

        let policy11 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    principal: principal!("*".to_string()),
                    actions: actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "myobject*".to_string(),
                    )]),
                    conditions: Default::default(),
                },
            ],
        };

        let cases = [
            (data1, Some(policy1), false),
            (data2, Some(policy2), false),
            (data3, Some(policy3), false),
            (data4, Some(policy4), false),
            (data5, Some(policy5), false),
            (data6, Some(policy6), false),
            (data7, Some(policy7), false),
            (data8, Some(policy8), false),
            // Invalid version error.
            (data9, None, true),
            // Duplicate statement success, duplicate statement removed.
            (data10, Some(policy10), false),
            // Duplicate statement success (Effect differs).
            (data11, Some(policy11), false),
        ];

        for (data, expected_result, expect_err) in cases {
            let result = serde_json::from_str::<Policy>(data);

            match result {
                Ok(result) => {
                    if let Some(expected_result) = expected_result {
                        assert!(result == expected_result);
                    }
                }
                Err(_) => assert!(expect_err),
            }
        }

        Ok(())
    }

    #[test]
    fn test_policy_validate() -> anyhow::Result<()> {
        let policy1 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: Default::default(),
            }],
        };

        let func1 = condition::new_null_func(
            condition::S3X_AMZ_COPY_SOURCE,
            condition::ValueSet::new(vec![condition::Value::Bool(true)]),
        )?;

        let func2 = condition::new_null_func(
            condition::S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            condition::ValueSet::new(vec![condition::Value::Bool(false)]),
        )?;

        let policy2 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                principal: principal!("*".to_string()),
                actions: actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "myobject*".to_string(),
                )]),
                conditions: condition::Functions::new(vec![func1, func2]),
            }],
        };

        let cases = [
            (&policy1, "mybucket", false),
            (&policy2, "yourbucket", true),
            (&policy1, "yourbucket", true),
        ];

        for (policy, bucket_name, expect_err) in cases {
            if expect_err {
                assert_err!(policy.validate(bucket_name));
            } else {
                assert_ok!(policy.validate(bucket_name));
            }
        }

        Ok(())
    }

    #[test]
    fn test_policy_merge() {
        let cases = [
            r#"{
                "Version": "2012-10-17",
                "ID": "S3PolicyId1",
                "Statement": [
                    {
                        "Sid": "statement1",
                        "Effect": "Deny",
                        "Principal": "*",
                        "Action":["s3:GetObject", "s3:PutObject"],
                        "Resource": "arn:aws:s3:::awsexamplebucket1/*"
                    }
                ]
            }"#,
            r#"{
                "Version": "2012-10-17",
                "ID": "S3PolicyId1",
                "Statement": [
                    {
                        "Sid": "statement1",
                        "Effect": "Allow",
                        "Principal": "*",
                        "Action":"s3:GetObject",
                        "Resource": "arn:aws:s3:::awsexamplebucket1/*",
                        "Condition" : {
                            "IpAddress" : {
                                "aws:SourceIp": "192.0.2.0/24"
                            },
                            "NotIpAddress" : {
                                "aws:SourceIp": "192.0.2.188/32"
                            }
                        }
                    }
                ]
            }"#,
            r#"{
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Sid": "cross-account permission to user in your own account",
                        "Effect": "Allow",
                        "Principal": {
                            "AWS": "arn:aws:iam::123456789012:user/Dave"
                        },
                        "Action": "s3:PutObject",
                        "Resource": "arn:aws:s3:::awsexamplebucket1/*"
                    },
                    {
                        "Sid": "Deny your user permission to upload object if copy source is not /bucket/folder",
                        "Effect": "Deny",
                        "Principal": {
                            "AWS": "arn:aws:iam::123456789012:user/Dave"
                        },
                        "Action": "s3:PutObject",
                        "Resource": "arn:aws:s3:::awsexamplebucket1/*",
                        "Condition": {
                            "StringNotLike": {
                                "s3:x-amz-copy-source": "awsexamplebucket1/public/*"
                            }
                        }
                    }
                ]
            }"#,
        ];

        for data in cases {
            let result = assert_ok!(serde_json::from_str::<Policy>(data));

            let merged_policy = result.merge(&result);

            let j = assert_ok!(serde_json::to_string(&merged_policy));

            let merged_policy = assert_ok!(serde_json::from_str::<Policy>(&j));

            assert!(result.statements == merged_policy.statements);
        }
    }
}
