use std::collections::HashMap;
use std::fmt;

use anyhow::bail;

use super::super::Valid;
use super::*;
use crate::strset::StringSet;

#[derive(Clone)]
pub(super) struct BinaryEqualsFunc<'a> {
    key: Key<'a>,
    values: StringSet,
}

impl<'a> fmt::Display for BinaryEqualsFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", BINARY_EQUALS, self.key, self.values)
    }
}

impl<'a> Function for BinaryEqualsFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        let mut v = values.get(&canonical_key(self.key.name()));
        if v.is_none() {
            v = values.get(self.key.name());
        }
        match v {
            Some(v) => {
                let fvalues = self.values.apply_fn(subst_func_from_values(values.clone()));
                !fvalues
                    .intersection(&StringSet::from_vec(v.clone()))
                    .is_empty()
            }
            None => false,
        }
    }

    fn key(&self) -> Key<'a> {
        self.key.clone()
    }

    fn name(&self) -> Name<'a> {
        BINARY_EQUALS
    }

    fn to_map(&self) -> HashMap<Key<'a>, ValueSet> {
        let mut map = HashMap::new();
        if !self.key.is_valid() {
            return map;
        }
        let values = ValueSet::new(
            self.values
                .as_slice()
                .iter()
                .map(|&v| Value::String(base64::encode(v)))
                .collect(),
        );
        map.insert(self.key.clone(), values);
        map
    }
}

pub(super) fn new_binary_equals_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    let value_strs = values_to_string_slice(BINARY_EQUALS, values)?;
    let mut set = StringSet::from_vec(value_strs);
    validate_binary_equals_values(BINARY_EQUALS, key.clone(), &mut set)?;
    Ok(Box::new(BinaryEqualsFunc { key, values: set }))
}

fn validate_binary_equals_values(
    name: Name,
    key: Key,
    values: &mut StringSet,
) -> anyhow::Result<()> {
    for s in values.to_vec() {
        let s_bytes = base64::decode(&s)?;
        values.remove(&s);
        let s = std::str::from_utf8(&s_bytes)?;

        match key {
            S3X_AMZ_COPY_SOURCE => {
                let (bucket, object) = path_to_bucket_and_object(s);
                if object.is_empty() {
                    bail!(
                        "invalid value '{}' for '{}' for {} condition",
                        s,
                        S3X_AMZ_COPY_SOURCE,
                        name
                    );
                }
                crate::s3utils::check_valid_bucket_name(bucket)?;
            }
            S3X_AMZ_SERVER_SIDE_ENCRYPTION | S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM => {
                if s != "AES256" {
                    bail!(
                        "invalid value '{}' for '{}' for {} condition",
                        s,
                        S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                        name
                    );
                }
            }
            S3X_AMZ_METADATA_DIRECTIVE => {
                if s != "COPY" && s != "REPLACE" {
                    bail!(
                        "invalid value '{}' for '{}' for {} condition",
                        s,
                        S3X_AMZ_METADATA_DIRECTIVE,
                        name
                    );
                }
            }
            S3X_AMZ_CONTENT_SHA256 => {
                if s.is_empty() {
                    bail!(
                        "invalid empty value for '{}' for {} condition",
                        S3X_AMZ_CONTENT_SHA256,
                        name
                    );
                }
            }
            _ => {}
        }

        values.add(s.to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_equals_func_evaluate() -> anyhow::Result<()> {
        let func1 = new_binary_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String(base64::encode("mybucket/myobject"))]),
        )?;

        let func2 = new_binary_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String(base64::encode("AES256"))]),
        )?;

        let func3 = new_binary_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String(base64::encode("REPLACE"))]),
        )?;

        let func4 = new_binary_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String(base64::encode("eu-west-1"))]),
        )?;

        let cases = [
            (
                &func1,
                HashMap::from([(
                    "x-amz-copy-source".to_string(),
                    vec!["mybucket/myobject".to_string()],
                )]),
                true,
            ),
            (
                &func1,
                HashMap::from([(
                    "x-amz-copy-source".to_string(),
                    vec!["yourbucket/myobject".to_string()],
                )]),
                false,
            ),
            (&func1, HashMap::new(), false),
            (
                &func1,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                false,
            ),
            (
                &func2,
                HashMap::from([(
                    "x-amz-server-side-encryption".to_string(),
                    vec!["AES256".to_string()],
                )]),
                true,
            ),
            (&func2, HashMap::new(), false),
            (
                &func2,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                false,
            ),
            (
                &func3,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["REPLACE".to_string()],
                )]),
                true,
            ),
            (
                &func3,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["COPY".to_string()],
                )]),
                false,
            ),
            (&func3, HashMap::new(), false),
            (
                &func3,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                false,
            ),
            (
                &func4,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["eu-west-1".to_string()],
                )]),
                true,
            ),
            (
                &func4,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["us-west-1".to_string()],
                )]),
                false,
            ),
            (&func4, HashMap::new(), false),
            (
                &func4,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                false,
            ),
        ];

        for (key, values, expected_result) in cases {
            let result = key.evaluate(&values);

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_binary_equals_func_key() -> anyhow::Result<()> {
        let func1 = new_binary_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String(base64::encode("mybucket/myobject"))]),
        )?;

        let func2 = new_binary_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String(base64::encode("AES256"))]),
        )?;

        let func3 = new_binary_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String(base64::encode("REPLACE"))]),
        )?;

        let func4 = new_binary_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String(base64::encode("eu-west-1"))]),
        )?;

        let cases = [
            (func1, S3X_AMZ_COPY_SOURCE),
            (func2, S3X_AMZ_SERVER_SIDE_ENCRYPTION),
            (func3, S3X_AMZ_METADATA_DIRECTIVE),
            (func4, S3_LOCATION_CONSTRAINT),
        ];

        for (key, expected_result) in cases {
            let result = key.key();

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_binary_equals_func_to_map() -> anyhow::Result<()> {
        let func1 = new_binary_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String(base64::encode("mybucket/myobject"))]),
        )?;

        let res1 = HashMap::from([(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String(base64::encode("mybucket/myobject"))]),
        )]);

        let func2 = new_binary_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String(base64::encode("mybucket/myobject")),
                Value::String(base64::encode("yourbucket/myobject")),
            ]),
        )?;

        let res2 = HashMap::from([(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String(base64::encode("mybucket/myobject")),
                Value::String(base64::encode("yourbucket/myobject")),
            ]),
        )]);

        let func3 = new_binary_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String(base64::encode("AES256"))]),
        )?;

        let res3 = HashMap::from([(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String(base64::encode("AES256"))]),
        )]);

        let func4 = new_binary_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String(base64::encode("REPLACE"))]),
        )?;

        let res4 = HashMap::from([(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String(base64::encode("REPLACE"))]),
        )]);

        let func5 = new_binary_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String(base64::encode("REPLACE")),
                Value::String(base64::encode("COPY")),
            ]),
        )?;

        let res5 = HashMap::from([(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String(base64::encode("REPLACE")),
                Value::String(base64::encode("COPY")),
            ]),
        )]);

        let func6 = new_binary_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String(base64::encode("eu-west-1"))]),
        )?;

        let res6 = HashMap::from([(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String(base64::encode("eu-west-1"))]),
        )]);

        let func7 = new_binary_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String(base64::encode("eu-west-1")),
                Value::String(base64::encode("us-west-1")),
            ]),
        )?;

        let res7 = HashMap::from([(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String(base64::encode("eu-west-1")),
                Value::String(base64::encode("us-west-1")),
            ]),
        )]);

        let cases = [
            (func1, res1),
            (func2, res2),
            (func3, res3),
            (func4, res4),
            (func5, res5),
            (func6, res6),
            (func7, res7),
        ];

        for (key, expected_result) in cases {
            let result = key.to_map();

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {:?}, got, {:?}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_new_binary_equals_func() -> anyhow::Result<()> {
        let func1 = new_binary_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String(base64::encode("mybucket/myobject"))]),
        )?;

        let func2 = new_binary_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String(base64::encode("mybucket/myobject")),
                Value::String(base64::encode("yourbucket/myobject")),
            ]),
        )?;

        let func3 = new_binary_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String(base64::encode("AES256"))]),
        )?;

        let func4 = new_binary_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String(base64::encode("REPLACE"))]),
        )?;

        let func5 = new_binary_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String(base64::encode("REPLACE")),
                Value::String(base64::encode("COPY")),
            ]),
        )?;

        let func6 = new_binary_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String(base64::encode("eu-west-1"))]),
        )?;

        let func7 = new_binary_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String(base64::encode("eu-west-1")),
                Value::String(base64::encode("us-west-1")),
            ]),
        )?;

        let cases = [
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![Value::String(base64::encode("mybucket/myobject"))]),
                Some(func1),
                false,
            ),
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![
                    Value::String(base64::encode("mybucket/myobject")),
                    Value::String(base64::encode("yourbucket/myobject")),
                ]),
                Some(func2),
                false,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String(base64::encode("AES256"))]),
                Some(func3),
                false,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String(base64::encode("REPLACE"))]),
                Some(func4),
                false,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![
                    Value::String(base64::encode("REPLACE")),
                    Value::String(base64::encode("COPY")),
                ]),
                Some(func5),
                false,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![Value::String(base64::encode("eu-west-1"))]),
                Some(func6),
                false,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![
                    Value::String(base64::encode("eu-west-1")),
                    Value::String(base64::encode("us-west-1")),
                ]),
                Some(func7),
                false,
            ),
            // Unsupported value error.
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![
                    Value::String(base64::encode("mybucket/myobject")),
                    Value::Int(7),
                ]),
                None,
                true,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String(base64::encode("AES256")), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![
                    Value::String(base64::encode("REPLACE")),
                    Value::Int(7),
                ]),
                None,
                true,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![
                    Value::String(base64::encode("eu-west-1")),
                    Value::Int(7),
                ]),
                None,
                true,
            ),
            // Invalid value error.
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![Value::String(base64::encode("mybucket"))]),
                None,
                true,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String(base64::encode("SSE-C"))]),
                None,
                true,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String(base64::encode("DUPLICATE"))]),
                None,
                true,
            ),
        ];

        for (key, values, expected_result, expect_err) in cases {
            let result = new_binary_equals_func(key.clone(), values);

            match result {
                Ok(result) => {
                    if let Some(expected_result) = expected_result {
                        assert_eq!(
                            result.to_string(),
                            expected_result.to_string(),
                            "key: '{}', expected: {}, got: {}",
                            key,
                            expected_result,
                            result
                        );
                    } else {
                        assert!(expect_err, "expect an error");
                    }
                }
                Err(_) => assert!(expect_err, "expect an error"),
            }
        }

        Ok(())
    }
}
