use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::*;
use crate::strset::StringSet;

// Policy principal.
#[derive(Eq, PartialEq, Clone, Debug, Default)]
pub struct Principal {
    pub aws: StringSet,
}

impl Valid for Principal {
    fn is_valid(&self) -> bool {
        !self.aws.is_empty()
    }
}

impl Principal {
    // Matches given principal is wildcard matching with Principal.
    pub fn is_match(&self, principal: &str) -> bool {
        self.aws
            .iter()
            .any(|p| crate::wildcard::match_wildcard_simple(p, principal))
    }

    pub fn intersection(&self, other: &StringSet) -> StringSet {
        self.aws.intersection(other)
    }
}

impl Serialize for Principal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if !self.is_valid() {
            return Err(S::Error::custom("invalid principal"));
        }
        let mut p = serializer.serialize_struct("Principal", 1)?;
        p.serialize_field("AWS", &self.aws)?;
        p.end()
    }
}

impl<'de> Deserialize<'de> for Principal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PrincipalVisitor;
        impl<'de> Visitor<'de> for PrincipalVisitor {
            type Value = Principal;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a principal")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v != "*" {
                    return Err(E::custom(format!("invalid principal '{}'", v)));
                }
                Ok(Principal {
                    aws: StringSet::from_slice(&["*"]),
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                use serde::de::Error;
                if let Ok(Some((k, v))) = map.next_entry::<&str, StringSet>() {
                    if k != "AWS" {
                        return Err(A::Error::custom(format!("invalid principal field '{}'", k)));
                    }
                    match map.next_key::<&str>() {
                        Ok(None) => {}
                        _ => {
                            return Err(A::Error::custom("invalid principal field"));
                        }
                    }
                    return Ok(Principal { aws: v });
                }
                return Err(A::Error::custom("invalid principal"));
            }
        }

        deserializer.deserialize_any(PrincipalVisitor)
    }
}

#[macro_export]
macro_rules! principal {
    ($($e:expr),*) => {{
        use crate::strset::StringSet;
        let mut set = StringSet::default();
        $(
            set.add($e);
        )*
        Principal {
            aws: set,
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_principal_is_valid() {
        let cases = [
            (principal!("*".to_owned()), true),
            (
                principal!("arn:aws:iam::AccountNumber:root".to_owned()),
                true,
            ),
            (principal!(), false),
        ];

        for (principal, expected_result) in cases {
            let result = principal.is_valid();

            assert_eq!(result, expected_result);
        }
    }

    #[test]
    fn test_principal_intersection() {
        let cases = [
            (
                principal!("*".to_owned()),
                principal!("*".to_owned()),
                StringSet::from_slice(&["*"]),
            ),
            (
                principal!("arn:aws:iam::AccountNumber:root".to_owned()),
                principal!("arn:aws:iam::AccountNumber:myuser".to_owned()),
                StringSet::new(),
            ),
            (
                principal!("".to_owned()),
                principal!("*".to_owned()),
                StringSet::new(),
            ),
        ];

        for (principal, to_intersect, expected_result) in cases {
            let result = principal.intersection(&to_intersect.aws);

            assert_eq!(
                result, expected_result,
                "principal: {:?}, expected: {}, got: {}",
                principal, expected_result, result
            );
        }
    }

    #[test]
    fn test_principal_match() {
        let cases = [
            (principal!("*".to_owned()), "AccountNumber", true),
            (
                principal!("arn:aws:iam:*".to_owned()),
                "arn:aws:iam::AccountNumber:root",
                true,
            ),
            (
                principal!("arn:aws:iam::AccountNumber:*".to_owned()),
                "arn:aws:iam::TestAccountNumber:root",
                false,
            ),
        ];

        for (principal, data, expected_result) in cases {
            let result = principal.is_match(data);

            assert_eq!(result, expected_result);
        }
    }

    #[test]
    fn test_principal_serialize_json() {
        let cases = vec![
            (principal!("*".to_owned()), r#"{"AWS":["*"]}"#, false),
            (
                principal!("arn:aws:iam::AccountNumber:*".to_owned()),
                r#"{"AWS":["arn:aws:iam::AccountNumber:*"]}"#,
                false,
            ),
            (principal!(), "", true),
        ];
        for (principal, expected_result, expect_err) in cases {
            match serde_json::to_string(&principal) {
                Ok(result) => {
                    assert!(!expect_err);
                    assert_eq!(result, expected_result);
                }
                Err(_) => {
                    assert!(principal.aws.is_empty());
                    assert!(expect_err);
                }
            }
        }
    }

    #[test]
    fn test_principal_deserialize_json() {
        let cases = vec![
            (r#""*""#, principal!("*".to_owned()), false),
            (r#"{"AWS": "*"}"#, principal!("*".to_owned()), false),
            (
                r#"{"AWS": "arn:aws:iam::AccountNumber:*"}"#,
                principal!("arn:aws:iam::AccountNumber:*".to_owned()),
                false,
            ),
            (
                r#"{"aws": "arn:aws:iam::AccountNumber:*"}"#,
                principal!(),
                true,
            ),
            (
                r#"{"AWS": "arn:aws:iam::AccountNumber:*", "unknown": ""}"#,
                principal!(),
                true,
            ),
            (r#""arn:aws:iam::AccountNumber:*""#, principal!(), true),
            (
                r#"["arn:aws:iam::AccountNumber:*", "arn:aws:iam:AnotherAccount:*"]"#,
                principal!(),
                true,
            ),
            (
                r#"{"AWS": ["arn:aws:iam::AccountNumber:*", "arn:aws:iam:AnotherAccount:*"]}"#,
                principal!(
                    "arn:aws:iam::AccountNumber:*".to_owned(),
                    "arn:aws:iam:AnotherAccount:*".to_owned()
                ),
                false,
            ),
        ];
        for (data, expected_result, expect_err) in cases {
            match serde_json::from_str::<Principal>(data) {
                Ok(result) => {
                    assert!(!expect_err);
                    assert_eq!(result, expected_result);
                }
                Err(e) => {
                    assert!(expect_err);
                }
            }
        }
    }
}
