use std::collections::HashMap;
use std::fmt;

use anyhow::bail;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::*;
use crate::bucket::policy as bpolicy;
use crate::bucket::policy::condition;
use crate::jwt::MapClaims;
use crate::strset::StringSet;

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
    pub claims: MapClaims,
    pub deny_only: bool,
}

impl<'a> Args<'a> {
    pub fn get_policies(&self, policy_claim_name: &str) -> Option<StringSet> {
        get_policies_from_claims(&self.claims, policy_claim_name)
    }
}

pub fn get_policies_from_claims(claims: &MapClaims, policy_claim_name: &str) -> Option<StringSet> {
    let mut set = StringSet::new();
    match claims.lookup(policy_claim_name) {
        serde_json::Value::String(s) => {
            for mut pname in s.split(',').collect::<Vec<&str>>() {
                pname = pname.trim();
                if pname.is_empty() {
                    // Ignore any empty strings, considerate
                    // towards some user errors.
                    continue;
                }
                set.add(pname.to_owned());
            }
        }
        serde_json::Value::Array(array) => {
            for v in array {
                if let serde_json::Value::String(s) = v {
                    for mut pname in s.split(',').collect::<Vec<&str>>() {
                        pname = pname.trim();
                        if pname.is_empty() {
                            // Ignore any empty strings, considerate
                            // towards some user errors.
                            continue;
                        }
                        set.add(pname.to_owned());
                    }
                }
            }
        }
        _ => {
            return None;
        }
    }
    Some(set)
}

// IAM policy.
pub struct Policy<'a, 'b> {
    pub id: bpolicy::ID,
    pub version: String,
    pub statements: Vec<Statement<'a, 'b>>,
}

impl<'a, 'b> Policy<'a, 'b> {
    pub fn match_resource(&self, resource: &str) -> bool {
        self.statements
            .iter()
            .any(|s| s.resources.is_match_resource(resource))
    }

    // Checks given policy args is allowed to continue the Rest API.
    pub fn is_allowed(&self, args: &Args) -> bool {
        // Check all deny statements. If any one statement denies, return false.
        for statement in &self.statements {
            if statement.effect == bpolicy::DENY && !statement.is_allowed(args) {
                return false;
            }
        }
        // Applied any 'Deny' only policies, if we have
        // reached here it means that there were no 'Deny'
        // policies - this function mainly used for
        // specific scenarios where we only want to validate
        // 'Deny' only policies.
        if args.deny_only {
            return true;
        }
        // For owner, its allowed by default.
        if args.is_owner {
            return true;
        }
        // Check all allow statements. If any one statement allows, return true.
        for statement in &self.statements {
            if statement.effect == bpolicy::ALLOW && statement.is_allowed(args) {
                return true;
            }
        }
        false
    }

    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    // Validates all statements are for given bucket or not.
    pub fn validate(&self) -> anyhow::Result<()> {
        self.is_valid()
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
        self.version == other.version && self.statements == other.statements
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
    use crate::bucket::policy::{ALLOW, DENY};
    use crate::iam_actionset;
    use crate::utils::assert::*;
    use crate::utils::{self, DateTime, DateTimeFormatExt};

    #[test]
    fn test_get_policies_from_claims() {
        let attrs = r#"{
            "exp": 1594690452,
            "iat": 1594689552,
            "auth_time": 1594689552,
            "jti": "18ed05c9-2c69-45d5-a33f-8c94aca99ad5",
            "iss": "http://localhost:8080/auth/realms/minio",
            "aud": "account",
            "sub": "7e5e2f30-1c97-4616-8623-2eae14dee9b1",
            "typ": "ID",
            "azp": "account",
            "nonce": "66ZoLzwJbjdkiedI",
            "session_state": "3df7b526-5310-4038-9f35-50ecd295a31d",
            "acr": "1",
            "upn": "harsha",
            "address": {},
            "email_verified": false,
            "groups": [
                "offline_access"
            ],
            "preferred_username": "harsha",
            "policy": [
                "readwrite",
                "readwrite,readonly",
                "readonly",
                ""
            ]
        }"#;

        let claims = assert_ok!(serde_json::from_str::<MapClaims>(attrs));

        let set = get_policies_from_claims(&claims, "policy").unwrap();

        assert!(!set.is_empty(), "no policies were found in policy claim");
    }

    #[test]
    fn test_policy_is_allowed() -> anyhow::Result<()> {
        let policy1 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                actions: iam_actionset!(GET_BUCKET_LOCATION_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
                conditions: condition::Functions::default(),
            }],
        };

        let policy2 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: condition::Functions::default(),
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
                actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
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
                actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: condition::Functions::new(vec![func1.clone()]),
            }],
        };

        let anon_get_bucket_location_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: Action::from(GET_BUCKET_LOCATION_ACTION),
            bucket_name: "mybucket".to_string(),
            condition_values: HashMap::default(),
            is_owner: false,
            object_name: "".to_string(),
            claims: MapClaims::default(),
            deny_only: false,
        };

        let anon_put_object_action_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: Action::from(PUT_OBJECT_ACTION),
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
            claims: MapClaims::default(),
            deny_only: false,
        };

        let anon_get_object_action_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: Action::from(GET_OBJECT_ACTION),
            bucket_name: "mybucket".to_string(),
            condition_values: HashMap::default(),
            is_owner: false,
            object_name: "myobject".to_string(),
            claims: MapClaims::default(),
            deny_only: false,
        };

        let get_bucket_location_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: Action::from(GET_BUCKET_LOCATION_ACTION),
            bucket_name: "mybucket".to_string(),
            condition_values: HashMap::default(),
            is_owner: false,
            object_name: "".to_string(),
            claims: MapClaims::default(),
            deny_only: false,
        };

        let put_object_action_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: Action::from(PUT_OBJECT_ACTION),
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
            claims: MapClaims::default(),
            deny_only: false,
        };

        let get_object_action_args = Args {
            account_name: "Q3AM3UQ867SPQQA43P2F".to_string(),
            groups: vec![],
            action: Action::from(GET_OBJECT_ACTION),
            bucket_name: "mybucket".to_string(),
            condition_values: HashMap::default(),
            is_owner: false,
            object_name: "myobject".to_string(),
            claims: MapClaims::default(),
            deny_only: false,
        };

        let cases = [
            (&policy1, &anon_get_bucket_location_args, true),
            (&policy1, &anon_put_object_action_args, true),
            (&policy1, &anon_get_object_action_args, false),
            (&policy1, &get_bucket_location_args, true),
            (&policy1, &put_object_action_args, true),
            (&policy1, &get_object_action_args, false),
            (&policy2, &anon_get_bucket_location_args, false),
            (&policy2, &anon_put_object_action_args, true),
            (&policy2, &anon_get_object_action_args, true),
            (&policy2, &get_bucket_location_args, false),
            (&policy2, &put_object_action_args, true),
            (&policy2, &get_object_action_args, true),
            (&policy3, &anon_get_bucket_location_args, false),
            (&policy3, &anon_put_object_action_args, true),
            (&policy3, &anon_get_object_action_args, false),
            (&policy3, &get_bucket_location_args, false),
            (&policy3, &put_object_action_args, true),
            (&policy3, &get_object_action_args, false),
            (&policy4, &anon_get_bucket_location_args, false),
            (&policy4, &anon_put_object_action_args, false),
            (&policy4, &anon_get_object_action_args, false),
            (&policy4, &get_bucket_location_args, false),
            (&policy4, &put_object_action_args, false),
            (&policy4, &get_object_action_args, false),
        ];

        for (index, (policy, args, expected_result)) in cases.into_iter().enumerate() {
            let result = policy.is_allowed(args);

            assert_eq!(
                result,
                *expected_result,
                "case: {}, expected: {}, got: {}",
                index + 1,
                expected_result,
                result
            );
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
                actions: iam_actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: condition::Functions::default(),
            }],
        };

        let policy2 = Policy {
            id: "MyPolicyForMyBucket".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![],
        };

        let cases = [(policy1, false), (policy2, true)];

        for (index, (policy, expected_result)) in cases.iter().enumerate() {
            let result = policy.is_empty();

            assert_eq!(
                result,
                *expected_result,
                "case: {}, expected: {}, got: {}",
                index + 1,
                expected_result,
                result
            );
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
                actions: iam_actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: condition::Functions::default(),
            }],
        };

        let policy2 = Policy {
            id: "".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    actions: iam_actionset!(GET_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
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
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    actions: iam_actionset!(GET_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/yourobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
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
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1.clone()]),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
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
                actions: iam_actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: condition::Functions::default(),
            }],
        };

        let policy6 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
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
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
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
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
            ],
        };

        let cases = [
            (policy1, false),
            // Allowed duplicate principal.
            (policy2, false),
            // Allowed duplicate principal.
            (policy3, false),
            // Allowed duplicate principal, action and resource.
            (policy4, false),
            // Invalid version error.
            (policy5, true),
            // Invalid statement error.
            (policy6, true),
            // Duplicate statement different Effects.
            (policy7, false),
            // Duplicate statement same Effects, duplicate effect will be removed.
            (policy8, false),
        ];

        for (policy, expect_err) in cases {
            if !expect_err {
                assert_ok!(policy.is_valid());
            } else {
                assert_err!(policy.is_valid());
            }
        }

        Ok(())
    }

    #[test]
    fn test_policy_parse_config() {
        let policy1_location_constraint = r#"{
            "Version":"2012-10-17",
            "Statement":[
                {
                    "Sid":"statement1",
                    "Effect":"Allow",
                    "Action": "s3:CreateBucket",
                    "Resource": "arn:aws:s3:::*",
                    "Condition": {
                        "StringLike": {
                            "s3:LocationConstraint": "us-east-1"
                        }
                    }
                },
                {
                    "Sid":"statement2",
                    "Effect":"Deny",
                    "Action": "s3:CreateBucket",
                    "Resource": "arn:aws:s3:::*",
                    "Condition": {
                        "StringNotLike": {
                            "s3:LocationConstraint": "us-east-1"
                        }
                    }
                }
            ]
        }"#;

        let policy2_condition = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Sid": "statement1",
                    "Effect": "Allow",
                    "Action": "s3:GetObjectVersion",
                    "Resource": "arn:aws:s3:::test/HappyFace.jpg"
                },
                {
                    "Sid": "statement2",
                    "Effect": "Deny",
                    "Action": "s3:GetObjectVersion",
                    "Resource": "arn:aws:s3:::test/HappyFace.jpg",
                    "Condition": {
                        "StringNotEquals": {
                            "s3:versionid": "AaaHbAQitwiL_h47_44lRO2DDfLlBO5e"
                        }
                    }
                }
            ]
        }"#;

        let policy3_condition_action_regex = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Sid": "statement2",
                    "Effect": "Allow",
                    "Action": "s3:Get*",
                    "Resource": "arn:aws:s3:::test/HappyFace.jpg",
                    "Condition": {
                        "StringEquals": {
                            "s3:versionid": "AaaHbAQitwiL_h47_44lRO2DDfLlBO5e"
                        }
                    }
                }
            ]
        }"#;

        let policy4_condition_action = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Sid": "statement2",
                    "Effect": "Allow",
                    "Action": "s3:GetObject",
                    "Resource": "arn:aws:s3:::test/HappyFace.jpg",
                    "Condition": {
                        "StringEquals": {
                            "s3:versionid": "AaaHbAQitwiL_h47_44lRO2DDfLlBO5e"
                        }
                    }
                }
            ]
        }"#;

        let policy5_condition_current_time = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Action": [
                        "s3:Get*",
                        "s3:Put*"
                    ],
                    "Resource": [
                        "arn:aws:s3:::test/*"
                    ],
                    "Condition": {
                        "DateGreaterThan": {
                            "aws:CurrentTime": [
                                "2017-02-28T00:00:00Z"
                            ]
                        }
                    }
                }
            ]
        }"#;

        let policy5_condition_current_time_lesser = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Action": [
                        "s3:Get*",
                        "s3:Put*"
                    ],
                    "Resource": [
                        "arn:aws:s3:::test/*"
                    ],
                    "Condition": {
                        "DateLessThan": {
                            "aws:CurrentTime": [
                                "2017-02-28T00:00:00Z"
                            ]
                        }
                    }
                }
            ]
        }"#;

        let cases = [
            (
                policy1_location_constraint,
                true,
                Args {
                    account_name: "allowed".to_string(),
                    groups: vec![],
                    action: Action::from(CREATE_BUCKET_ACTION),
                    bucket_name: "test".to_string(),
                    condition_values: HashMap::from([(
                        "LocationConstraint".to_string(),
                        vec!["us-east-1".to_string()],
                    )]),
                    is_owner: false,
                    object_name: "".to_string(),
                    claims: MapClaims::default(),
                    deny_only: false,
                },
            ),
            (
                policy1_location_constraint,
                false,
                Args {
                    account_name: "disallowed".to_string(),
                    groups: vec![],
                    action: Action::from(CREATE_BUCKET_ACTION),
                    bucket_name: "test".to_string(),
                    condition_values: HashMap::from([(
                        "LocationConstraint".to_string(),
                        vec!["us-east-2".to_string()],
                    )]),
                    is_owner: false,
                    object_name: "".to_string(),
                    claims: MapClaims::default(),
                    deny_only: false,
                },
            ),
            (
                policy2_condition,
                true,
                Args {
                    account_name: "allowed".to_string(),
                    groups: vec![],
                    action: Action::from(GET_OBJECT_ACTION),
                    bucket_name: "test".to_string(),
                    condition_values: HashMap::from([(
                        "versionid".to_string(),
                        vec!["AaaHbAQitwiL_h47_44lRO2DDfLlBO5e".to_string()],
                    )]),
                    is_owner: false,
                    object_name: "HappyFace.jpg".to_string(),
                    claims: MapClaims::default(),
                    deny_only: false,
                },
            ),
            (
                policy2_condition,
                false,
                Args {
                    account_name: "disallowed".to_string(),
                    groups: vec![],
                    action: Action::from(GET_OBJECT_ACTION),
                    bucket_name: "test".to_string(),
                    condition_values: HashMap::from([(
                        "versionid".to_string(),
                        vec!["AaaHbAQitwiL_h47_44lRO2DDfLlBO5f".to_string()],
                    )]),
                    is_owner: false,
                    object_name: "HappyFace.jpg".to_string(),
                    claims: MapClaims::default(),
                    deny_only: false,
                },
            ),
            (
                policy3_condition_action_regex,
                true,
                Args {
                    account_name: "allowed".to_string(),
                    groups: vec![],
                    action: Action::from(GET_OBJECT_ACTION),
                    bucket_name: "test".to_string(),
                    condition_values: HashMap::from([(
                        "versionid".to_string(),
                        vec!["AaaHbAQitwiL_h47_44lRO2DDfLlBO5e".to_string()],
                    )]),
                    is_owner: false,
                    object_name: "HappyFace.jpg".to_string(),
                    claims: MapClaims::default(),
                    deny_only: false,
                },
            ),
            (
                policy3_condition_action_regex,
                false,
                Args {
                    account_name: "disallowed".to_string(),
                    groups: vec![],
                    action: Action::from(GET_OBJECT_ACTION),
                    bucket_name: "test".to_string(),
                    condition_values: HashMap::from([(
                        "versionid".to_string(),
                        vec!["AaaHbAQitwiL_h47_44lRO2DDfLlBO5f".to_string()],
                    )]),
                    is_owner: false,
                    object_name: "HappyFace.jpg".to_string(),
                    claims: MapClaims::default(),
                    deny_only: false,
                },
            ),
            (
                policy4_condition_action,
                true,
                Args {
                    account_name: "allowed".to_string(),
                    groups: vec![],
                    action: Action::from(GET_OBJECT_ACTION),
                    bucket_name: "test".to_string(),
                    condition_values: HashMap::from([(
                        "versionid".to_string(),
                        vec!["AaaHbAQitwiL_h47_44lRO2DDfLlBO5e".to_string()],
                    )]),
                    is_owner: false,
                    object_name: "HappyFace.jpg".to_string(),
                    claims: MapClaims::default(),
                    deny_only: false,
                },
            ),
            (
                policy5_condition_current_time,
                true,
                Args {
                    account_name: "allowed".to_string(),
                    groups: vec![],
                    action: Action::from(GET_OBJECT_ACTION),
                    bucket_name: "test".to_string(),
                    condition_values: HashMap::from([(
                        "CurrentTime".to_string(),
                        vec![utils::now().rfc3339()],
                    )]),
                    is_owner: false,
                    object_name: "HappyFace.jpg".to_string(),
                    claims: MapClaims::default(),
                    deny_only: false,
                },
            ),
            (
                policy5_condition_current_time_lesser,
                false,
                Args {
                    account_name: "disallowed".to_string(),
                    groups: vec![],
                    action: Action::from(GET_OBJECT_ACTION),
                    bucket_name: "test".to_string(),
                    condition_values: HashMap::from([(
                        "CurrentTime".to_string(),
                        vec![utils::now().rfc3339()],
                    )]),
                    is_owner: false,
                    object_name: "HappyFace.jpg".to_string(),
                    claims: MapClaims::default(),
                    deny_only: false,
                },
            ),
        ];

        for (data, allowed, args) in cases {
            let policy = assert_ok!(serde_json::from_str::<Policy>(data));

            assert_ok!(policy.validate());

            let result = policy.is_allowed(&args);
            assert_eq!(result, allowed);
        }
    }

    #[test]
    fn test_policy_deserialize_json_and_validate() -> anyhow::Result<()> {
        let data1 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Sid": "SomeId1",
                    "Effect": "Allow",
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
                actions: iam_actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                conditions: condition::Functions::default(),
            }],
        };

        let data2 = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Deny",
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
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    actions: iam_actionset!(GET_OBJECT_ACTION),
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
                    "Action": "s3:GetObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Allow",
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
                    actions: iam_actionset!(GET_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
            ],
        };

        let data4 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Allow",
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
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(GET_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
            ],
        };

        let data5 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Allow",
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
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/yourobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
            ],
        };

        let data6 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
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
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func1]),
                },
                Statement {
                    sid: "".to_string(),
                    effect: ALLOW,
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "/myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::new(vec![func2]),
                },
            ],
        };

        let data7 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
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
                actions: iam_actionset!(GET_BUCKET_LOCATION_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "".to_string(),
                )]),
                conditions: condition::Functions::default(),
            }],
        };

        let data8 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
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
                actions: iam_actionset!(GET_BUCKET_LOCATION_ACTION),
                resources: ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
                conditions: condition::Functions::default(),
            }],
        };

        let data9 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "17-10-2012",
            "Statement": [
                {
                    "Effect": "Allow",
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
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Allow",
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
                actions: iam_actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "myobject*".to_string(),
                )]),
                conditions: condition::Functions::default(),
            }],
        };

        let data11 = r#"{
            "ID": "MyPolicyForMyBucket1",
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Action": "s3:PutObject",
                    "Resource": "arn:aws:s3:::mybucket/myobject*"
                },
                {
                    "Effect": "Deny",
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
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
                Statement {
                    sid: "".to_string(),
                    effect: DENY,
                    actions: iam_actionset!(PUT_OBJECT_ACTION),
                    resources: ResourceSet::new(vec![Resource::new(
                        "mybucket".to_string(),
                        "myobject*".to_string(),
                    )]),
                    conditions: condition::Functions::default(),
                },
            ],
        };

        let cases = [
            (data1, Some(policy1), false, false),
            (data2, Some(policy2), false, false),
            (data3, Some(policy3), false, false),
            (data4, Some(policy4), false, false),
            (data5, Some(policy5), false, false),
            (data6, Some(policy6), false, false),
            (data7, Some(policy7), false, false),
            (data8, Some(policy8), false, false),
            // Invalid version error.
            (data9, None, true, false),
            // Duplicate statement success, duplicate statement is removed.
            (data10, Some(policy10), false, false),
            // Duplicate statement success (Effect differs).
            (data11, Some(policy11), false, false),
        ];

        for (data, expected_result, expect_deserialize_err, expect_validation_err) in cases {
            let result = serde_json::from_str::<Policy>(data);

            match result {
                Ok(result) => {
                    if let Some(expected_result) = expected_result {
                        assert!(result == expected_result);

                        if !expect_validation_err {
                            assert_ok!(result.validate());
                        } else {
                            assert_err!(result.validate());
                        }
                    }
                }
                Err(_) => assert!(expect_deserialize_err),
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
                actions: iam_actionset!(PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new("".to_string(), "".to_string())]),
                conditions: condition::Functions::default(),
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
                actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "myobject*".to_string(),
                )]),
                conditions: condition::Functions::new(vec![func1, func2]),
            }],
        };

        let policy3 = Policy {
            id: "MyPolicyForMyBucket1".to_string(),
            version: DEFAULT_VERSION.to_string(),
            statements: vec![Statement {
                sid: "".to_string(),
                effect: ALLOW,
                actions: iam_actionset!(GET_OBJECT_ACTION, PUT_OBJECT_ACTION),
                resources: ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "myobject*".to_string(),
                )]),
                conditions: condition::Functions::default(),
            }],
        };

        let cases = [(policy1, true), (policy2, true), (policy3, false)];

        for (policy, expect_err) in cases {
            if !expect_err {
                assert_ok!(policy.validate());
            } else {
                assert_err!(policy.validate());
            }
        }

        Ok(())
    }
}
