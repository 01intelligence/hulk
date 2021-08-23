use std::collections::HashMap;
use std::fmt;

use dyn_clone::DynClone;
use lazy_static::lazy_static;
use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};

use super::super::Valid;
use super::*;

// Condition function trait.
pub trait Function: fmt::Display + DynClone {
    // Evaluates this condition function with given values.
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool;

    // Returns condition key used in this function.
    fn key(&self) -> Key;

    // Returns condition name of this function.
    fn name(&self) -> Name;

    // Returns map representation of this function.
    fn to_map(&self) -> HashMap<Key, ValueSet>;
}

dyn_clone::clone_trait_object!(Function);

// List of functions.
#[derive(Clone, Default)]
pub struct Functions(Vec<Box<dyn Function>>);

impl Functions {
    pub fn new(functions: Vec<Box<dyn Function>>) -> Functions {
        Functions(functions)
    }

    // Evaluates all functions with given values map. Each function is evaluated
    // sequentially and next function is called only if current function succeeds.
    pub fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        self.0.iter().all(|f| f.evaluate(values))
    }

    // Returns list of keys used in all functions.
    pub fn keys(&self) -> KeySet {
        self.0.iter().map(|f| f.key()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl PartialEq for Functions {
    // Returns true if two Functions structures are equal.
    fn eq(&self, other: &Self) -> bool {
        self.0
            .iter()
            .all(|f| other.0.iter().any(|g| g.to_string() == f.to_string()))
    }
}

impl std::cmp::Eq for Functions {}

impl std::fmt::Display for Functions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut func_strings: Vec<String> = self.0.iter().map(|f| f.to_string()).collect();
        func_strings.sort_unstable();
        write!(f, "{:?}", func_strings)
    }
}

impl Serialize for Functions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut nm = HashMap::<Name, HashMap<Key, ValueSet>>::new();
        for f in &self.0 {
            let v = nm.entry(f.name()).or_default();
            v.extend(f.to_map());
        }
        let mut map = serializer.serialize_map(Some(nm.len()))?;
        for (k, v) in nm {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Functions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FunctionsVisitor;
        impl<'de> Visitor<'de> for FunctionsVisitor {
            type Value = Functions;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a condition value set")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                use serde::de::Error;
                let mut nm = HashMap::<Name, HashMap<Key, ValueSet>>::new();
                while let Some((k, v)) = map.next_entry()? {
                    nm.insert(k, v);
                }
                if nm.is_empty() {
                    return Err(A::Error::custom("condition must not be empty"));
                }
                let mut funcs = Functions::new(vec![]);
                for (name, args) in nm {
                    if !name.is_valid() {
                        return Err(A::Error::custom(format!(
                            "invalid condition name '{}'",
                            name
                        )));
                    }
                    for (key, values) in args {
                        if !key.is_valid() {
                            return Err(A::Error::custom(format!(
                                "invalid condition key '{}'",
                                key
                            )));
                        }
                        let vfn = match CONDITION_FUNC_MAP.get(&name) {
                            None => {
                                return Err(A::Error::custom(format!(
                                    "condition {} is not handled",
                                    name
                                )));
                            }
                            Some(vfn) => vfn,
                        };
                        let f = vfn(key, values).map_err(|e| A::Error::custom(format!("{}", e)))?;
                        funcs.0.push(f);
                    }
                }
                Ok(funcs)
            }
        }

        deserializer.deserialize_map(FunctionsVisitor)
    }
}

type NewFunction = fn(Key, ValueSet) -> anyhow::Result<Box<dyn Function + '_>>;

lazy_static! {
    static ref CONDITION_FUNC_MAP: HashMap<Name<'static>, NewFunction> = maplit::hashmap! {
        STRING_EQUALS => new_string_equals_func as NewFunction,
        STRING_NOT_EQUALS => new_string_not_equals_func as NewFunction,
        STRING_EQUALS_IGNORE_CASE => new_string_equals_ignore_case_func as NewFunction,
        STRING_NOT_EQUALS_IGNORE_CASE => new_string_not_equals_ignore_case_func as NewFunction,
        STRING_LIKE => new_string_like_func as NewFunction,
        STRING_NOT_LIKE => new_string_not_like_func as NewFunction,
        BOOLEAN => new_boolean_func as NewFunction,
        NULL => new_null_func as NewFunction,
        BINARY_EQUALS => new_binary_equals_func as NewFunction,
        IP_ADDRESS => new_ip_address_func as NewFunction,
        NOT_IP_ADDRESS => new_not_ip_address_func as NewFunction,
        NUMERIC_EQUALS => new_numeric_equals_func as NewFunction,
        NUMERIC_NOT_EQUALS => new_numeric_not_equals_func as NewFunction,
        NUMERIC_LESS_THAN => new_numeric_less_than_func as NewFunction,
        NUMERIC_LESS_THAN_EQUALS => new_numeric_less_than_equals_func as NewFunction,
        NUMERIC_GREATER_THAN => new_numeric_greater_than_func as NewFunction,
        NUMERIC_GREATER_THAN_EQUALS => new_numeric_greater_than_equals_func as NewFunction,
        DATE_EQUALS => new_date_equals_func as NewFunction,
        DATE_NOT_EQUALS => new_date_not_equals_func as NewFunction,
        DATE_LESS_THAN => new_date_less_than_func as NewFunction,
        DATE_LESS_THAN_EQUALS => new_date_less_than_equals_func as NewFunction,
        DATE_GREATER_THAN => new_date_greater_than_func as NewFunction,
        DATE_GREATER_THAN_EQUALS => new_date_greater_than_equals_func as NewFunction,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::assert::*;

    #[test]
    fn test_functions_evaluate() -> anyhow::Result<()> {
        let func1 = new_null_func(S3X_AMZ_COPY_SOURCE, ValueSet::new(vec![Value::Bool(true)]))?;

        let func2 = new_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;

        let func3 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func4 = new_string_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let funcs = Functions::new(vec![func1, func2, func3, func4]);

        let cases = [
            (
                &funcs,
                HashMap::from([
                    (
                        "x-amz-copy-source".to_string(),
                        vec!["mybucket/myobject".to_string()],
                    ),
                    ("SourceIp".to_string(), vec!["192.168.1.10".to_string()]),
                ]),
                false,
            ),
            (
                &funcs,
                HashMap::from([
                    (
                        "x-amz-copy-source".to_string(),
                        vec!["mybucket/myobject".to_string()],
                    ),
                    ("SourceIp".to_string(), vec!["192.168.1.10".to_string()]),
                    ("Refer".to_string(), vec!["http://example.org/".to_string()]),
                ]),
                false,
            ),
            (
                &funcs,
                HashMap::from([(
                    "x-amz-copy-source".to_string(),
                    vec!["mybucket/myobject".to_string()],
                )]),
                false,
            ),
            (
                &funcs,
                HashMap::from([("SourceIp".to_string(), vec!["192.168.1.10".to_string()])]),
                false,
            ),
            (
                &funcs,
                HashMap::from([
                    (
                        "x-amz-copy-source".to_string(),
                        vec!["mybucket/yourobject".to_string()],
                    ),
                    ("SourceIp".to_string(), vec!["192.168.1.10".to_string()]),
                ]),
                false,
            ),
            (
                &funcs,
                HashMap::from([
                    (
                        "x-amz-copy-source".to_string(),
                        vec!["mybucket/myobject".to_string()],
                    ),
                    ("SourceIp".to_string(), vec!["192.168.2.10".to_string()]),
                ]),
                false,
            ),
            (
                &funcs,
                HashMap::from([
                    (
                        "x-amz-copy-source".to_string(),
                        vec!["mybucket/myobject".to_string()],
                    ),
                    ("Refer".to_string(), vec!["http://example.org/".to_string()]),
                ]),
                false,
            ),
        ];

        for (key, values, expected_result) in cases {
            let result = funcs.evaluate(&values);

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_functions_keys() -> anyhow::Result<()> {
        let func1 = new_null_func(S3X_AMZ_COPY_SOURCE, ValueSet::new(vec![Value::Bool(true)]))?;
        let func2 = new_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;
        let func3 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;
        let func4 = new_string_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let cases = [(
            Functions::new(vec![func1, func2, func3, func4]),
            KeySet::from([S3X_AMZ_COPY_SOURCE, AWS_SOURCE_IP]),
        )];

        for (key, expected_result) in cases {
            let result = key.keys();

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {:?}, got: {:?}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_empty_functions_serialize_json() {
        let funcs = Functions::new(vec![]);
        let expected_result = "{}";
        let result = assert_ok!(serde_json::to_string(&funcs));
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_functions_serialize_json() -> anyhow::Result<()> {
        let func1 = new_string_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPL*".to_string())]),
        )?;

        let func2 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func3 = new_string_not_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func4 = new_not_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("10.1.10.0/24".to_string())]),
        )?;

        let func5 = new_string_not_like_func(
            S3X_AMZ_STORAGE_CLASS,
            ValueSet::new(vec![Value::String("STANDARD".to_string())]),
        )?;

        let func6 = new_null_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
            ValueSet::new(vec![Value::Bool(true)]),
        )?;

        let func7 = new_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![Value::String("192.168.1.0/24".to_string())]),
        )?;

        let res1 = r#"{"IpAddress":{"aws:SourceIp":["192.168.1.0/24"]},"NotIpAddress":{"aws:SourceIp":["10.1.10.0/24"]},"Null":{"s3:x-amz-server-side-encryption-customer-algorithm":[true]},"StringEquals":{"s3:x-amz-copy-source":["mybucket/myobject"]},"StringLike":{"s3:x-amz-metadata-directive":["REPL*"]},"StringNotEquals":{"s3:x-amz-server-side-encryption":["AES256"]},"StringNotLike":{"s3:x-amz-storage-class":["STANDARD"]}}"#;
        let res2 = r#"{"Null":{"s3:x-amz-server-side-encryption-customer-algorithm":[true]}}"#;

        let cases = [
            (
                Functions::new(vec![
                    func1,
                    func2,
                    func3,
                    func4,
                    func5,
                    func6.clone(),
                    func7,
                ]),
                res1,
            ),
            (Functions::new(vec![func6]), res2),
        ];

        for (key, expected_result) in cases {
            let result = assert_ok!(serde_json::to_string(&key));

            let result_de = serde_json::from_str::<Functions>(&result)?;
            let expected_result_de = serde_json::from_str::<Functions>(expected_result)?;
            assert_eq!(
                result_de.to_string(),
                expected_result_de.to_string(),
                "key: '{}', expected: {}, result: {}",
                key,
                expected_result,
                result
            );
        }

        Ok(())
    }

    #[test]
    fn test_functions_deserialize_json() -> anyhow::Result<()> {
        let case1 = r#"
            {
                "StringLike": {
                    "s3:x-amz-metadata-directive": "REPL*"
                },
                "StringEquals": {
                    "s3:x-amz-copy-source": "mybucket/myobject"
                },
                "StringNotEquals": {
                    "s3:x-amz-server-side-encryption": "AES256"
                },
                "NotIpAddress": {
                    "aws:SourceIp": [
                        "10.1.10.0/24",
                        "10.10.1.0/24"
                    ]
                },
                "StringNotLike": {
                    "s3:x-amz-storage-class": "STANDARD"
                },
                "Null": {
                    "s3:x-amz-server-side-encryption-customer-algorithm": true
                },
                "IpAddress": {
                    "aws:SourceIp": [
                        "192.168.1.0/24",
                        "192.168.2.0/24"
                    ]
                }
            }
        "#;
        let case2 = r#"
            {
                "Null": {
                    "s3:x-amz-server-side-encryption-customer-algorithm": true
                },
                "Null": {
                    "s3:x-amz-server-side-encryption-customer-algorithm": "true"
                }
            }
        "#;

        let case3 = r#"{}"#;

        let case4 = r#"
            {
                "StringLike": {
                    "s3:x-amz-metadata-directive": "REPL*"
                },
                "StringEquals": {
                    "s3:x-amz-copy-source": "mybucket/myobject",
                    "s3:prefix": [
                        "",
                        "home/"
                    ],
                    "s3:delimiter": [
                        "/"
                    ]
                },
                "StringNotEquals": {
                    "s3:x-amz-server-side-encryption": "AES256"
                },
                "NotIpAddress": {
                    "aws:SourceIp": [
                        "10.1.10.0/24",
                        "10.10.1.0/24"
                    ]
                },
                "StringNotLike": {
                    "s3:x-amz-storage-class": "STANDARD"
                },
                "Null": {
                    "s3:x-amz-server-side-encryption-customer-algorithm": true
                },
                "IpAddress": {
                    "aws:SourceIp": [
                        "192.168.1.0/24",
                        "192.168.2.0/24"
                    ]
                }
            }
        "#;

        let func1 = new_string_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPL*".to_string())]),
        )?;

        let func2 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func3 = new_string_not_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func4 = new_not_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![
                Value::String("10.1.10.0/24".to_string()),
                Value::String("10.10.1.0/24".to_string()),
            ]),
        )?;

        let func5 = new_string_not_like_func(
            S3X_AMZ_STORAGE_CLASS,
            ValueSet::new(vec![Value::String("STANDARD".to_string())]),
        )?;

        let func6 = new_null_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
            ValueSet::new(vec![Value::Bool(true)]),
        )?;

        let func7 = new_ip_address_func(
            AWS_SOURCE_IP,
            ValueSet::new(vec![
                Value::String("192.168.1.0/24".to_string()),
                Value::String("192.168.2.0/24".to_string()),
            ]),
        )?;

        let func2_1 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;
        let func2_2 = new_string_equals_func(
            S3_PREFIX,
            ValueSet::new(vec![
                Value::String("".to_string()),
                Value::String("home/".to_string()),
            ]),
        )?;
        let func2_3 = new_string_equals_func(
            S3_DELIMITER,
            ValueSet::new(vec![Value::String("/".to_string())]),
        )?;

        let cases = [
            (
                case1,
                Functions::new(vec![
                    func1.clone(),
                    func2,
                    func3.clone(),
                    func4.clone(),
                    func5.clone(),
                    func6.clone(),
                    func7.clone(),
                ]),
            ),
            (case2, Functions::new(vec![func6.clone()])),
            // (case3, Functions::new(vec![])),
            (
                case4,
                Functions::new(vec![
                    func1, func2_1, func2_2, func2_3, func3, func4, func5, func6, func7,
                ]),
            ),
        ];

        for (key, expected_result) in cases {
            let result = assert_ok!(serde_json::from_str::<Functions>(key));

            assert_eq!(
                result.to_string(),
                expected_result.to_string(),
                "key: '{}', expected: {}, got: {}",
                key,
                expected_result,
                result
            );
        }

        Ok(())
    }
}
