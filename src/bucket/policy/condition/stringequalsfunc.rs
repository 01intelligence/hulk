use std::collections::HashMap;
use std::fmt;

use anyhow::bail;

use super::super::Valid;
use super::*;
use crate::strset::StringSet;

// String equals function. It checks whether value by Key in given
// values map is in condition values.
// For example,
//   - if values = ["mybucket/foo"], at evaluate() it returns whether string
//     in value map for Key is in values.
#[derive(Clone)]
pub(super) struct StringEqualsFunc<'a> {
    key: Key<'a>,
    values: StringSet,
}

impl<'a> fmt::Display for StringEqualsFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", STRING_EQUALS, self.key, self.values)
    }
}

impl<'a> Function for StringEqualsFunc<'a> {
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
        STRING_EQUALS
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
                .map(|&v| Value::String(v.to_owned()))
                .collect(),
        );
        map.insert(self.key.clone(), values);
        map
    }
}

// String not equals function. It checks whether value by Key in
// given values is NOT in condition values.
// For example,
//   - if values = ["mybucket/foo"], at evaluate() it returns whether string
//     in value map for Key is NOT in values.
#[derive(Clone)]
pub(super) struct StringNotEqualsFunc<'a>(StringEqualsFunc<'a>);

impl<'a> fmt::Display for StringNotEqualsFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", STRING_NOT_EQUALS, self.0.key, self.0.values)
    }
}

impl<'a> Function for StringNotEqualsFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        !self.0.evaluate(values)
    }

    fn key(&self) -> Key<'_> {
        self.0.key()
    }

    fn name(&self) -> Name<'_> {
        STRING_NOT_EQUALS
    }

    fn to_map(&self) -> HashMap<Key<'_>, ValueSet> {
        self.0.to_map()
    }
}

pub(super) fn new_string_equals_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    let value_strs = values_to_string_slice(STRING_EQUALS, values)?;
    let set = StringSet::from_vec(value_strs);
    validate_string_equals_values(STRING_EQUALS, key.clone(), &set)?;
    Ok(Box::new(StringEqualsFunc { key, values: set }))
}

pub(super) fn new_string_not_equals_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    let value_strs = values_to_string_slice(STRING_NOT_EQUALS, values)?;
    let set = StringSet::from_vec(value_strs);
    validate_string_equals_values(STRING_NOT_EQUALS, key.clone(), &set)?;
    Ok(Box::new(StringNotEqualsFunc(StringEqualsFunc {
        key,
        values: set,
    })))
}

pub(super) fn validate_string_equals_values(
    name: Name,
    key: Key,
    values: &StringSet,
) -> anyhow::Result<()> {
    for s in values.as_slice() {
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
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_equals_func_evaluate() -> anyhow::Result<()> {
        let func1 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func2 = new_string_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func3 = new_string_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let func4 = new_string_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
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
            (&func1, HashMap::<String, Vec<String>>::new(), false),
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
            (&func2, HashMap::<String, Vec<String>>::new(), false),
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
            (&func3, HashMap::<String, Vec<String>>::new(), false),
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
                    vec!["us-east-1".to_string()],
                )]),
                false,
            ),
            (&func4, HashMap::<String, Vec<String>>::new(), false),
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
    fn test_string_equals_func_key() -> anyhow::Result<()> {
        let func1 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func2 = new_string_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func3 = new_string_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let func4 = new_string_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
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
    fn test_string_equals_func_to_map() -> anyhow::Result<()> {
        let func1 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let res1 = HashMap::from([(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )]);

        let func2 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/myobject".to_string()),
                Value::String("yourbucket/myobject".to_string()),
            ]),
        )?;

        let res2 = HashMap::from([(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/myobject".to_string()),
                Value::String("yourbucket/myobject".to_string()),
            ]),
        )]);

        let func3 = new_string_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let res3 = HashMap::from([(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )]);

        let func4 = new_string_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let res4 = HashMap::from([(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )]);

        let func5 = new_string_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPLACE".to_string()),
                Value::String("COPY".to_string()),
            ]),
        )?;

        let res5 = HashMap::from([(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPLACE".to_string()),
                Value::String("COPY".to_string()),
            ]),
        )]);

        let func6 = new_string_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
        )?;

        let res6 = HashMap::from([(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
        )]);

        let func7 = new_string_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-1".to_string()),
                Value::String("us-west-1".to_string()),
            ]),
        )?;

        let res7 = HashMap::from([(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-1".to_string()),
                Value::String("us-west-1".to_string()),
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
                "key: '{}', expected: {:?}, got: {:?}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_string_not_equals_func_evaluate() -> anyhow::Result<()> {
        let func1 = new_string_not_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func2 = new_string_not_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func3 = new_string_not_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let func4 = new_string_not_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
        )?;

        let cases = [
            (
                &func1,
                HashMap::from([(
                    "x-amz-copy-source".to_string(),
                    vec!["mybucket/myobject".to_string()],
                )]),
                false,
            ),
            (
                &func1,
                HashMap::from([(
                    "x-amz-copy-source".to_string(),
                    vec!["yourbucket/myobject".to_string()],
                )]),
                true,
            ),
            (&func1, HashMap::<String, Vec<String>>::new(), true),
            (
                &func1,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func2,
                HashMap::from([(
                    "x-amz-server-side-encryption".to_string(),
                    vec!["AES256".to_string()],
                )]),
                false,
            ),
            (&func2, HashMap::<String, Vec<String>>::new(), true),
            (
                &func2,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func3,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["REPLACE".to_string()],
                )]),
                false,
            ),
            (
                &func3,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["COPY".to_string()],
                )]),
                true,
            ),
            (&func3, HashMap::<String, Vec<String>>::new(), true),
            (
                &func3,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func4,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["eu-west-1".to_string()],
                )]),
                false,
            ),
            (
                &func4,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["us-east-1".to_string()],
                )]),
                true,
            ),
            (&func4, HashMap::<String, Vec<String>>::new(), true),
            (
                &func4,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
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
    fn test_string_not_equals_func_key() -> anyhow::Result<()> {
        let func1 = new_string_not_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func2 = new_string_not_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func3 = new_string_not_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let func4 = new_string_not_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
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
    fn test_string_not_equals_func_to_map() -> anyhow::Result<()> {
        let func1 = new_string_not_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let res1 = HashMap::from([(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )]);

        let func2 = new_string_not_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/myobject".to_string()),
                Value::String("yourbucket/myobject".to_string()),
            ]),
        )?;

        let res2 = HashMap::from([(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/myobject".to_string()),
                Value::String("yourbucket/myobject".to_string()),
            ]),
        )]);

        let func3 = new_string_not_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let res3 = HashMap::from([(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )]);

        let func4 = new_string_not_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let res4 = HashMap::from([(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )]);

        let func5 = new_string_not_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPLACE".to_string()),
                Value::String("COPY".to_string()),
            ]),
        )?;

        let res5 = HashMap::from([(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPLACE".to_string()),
                Value::String("COPY".to_string()),
            ]),
        )]);

        let func6 = new_string_not_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
        )?;

        let res6 = HashMap::from([(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
        )]);

        let func7 = new_string_not_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-1".to_string()),
                Value::String("us-west-1".to_string()),
            ]),
        )?;

        let res7 = HashMap::from([(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-1".to_string()),
                Value::String("us-west-1".to_string()),
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
                "key: '{}', expected: {:?}, got: {:?}",
                key, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_new_string_equals_func() -> anyhow::Result<()> {
        let func1 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func2 = new_string_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/myobject".to_string()),
                Value::String("yourbucket/myobject".to_string()),
            ]),
        )?;

        let func3 = new_string_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func4 = new_string_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let func5 = new_string_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPLACE".to_string()),
                Value::String("COPY".to_string()),
            ]),
        )?;

        let func6 = new_string_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
        )?;

        let func7 = new_string_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-1".to_string()),
                Value::String("us-west-1".to_string()),
            ]),
        )?;

        let cases = [
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
                Some(func1),
                false,
            ),
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![
                    Value::String("mybucket/myobject".to_string()),
                    Value::String("yourbucket/myobject".to_string()),
                ]),
                Some(func2),
                false,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String("AES256".to_string())]),
                Some(func3),
                false,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String("REPLACE".to_string())]),
                Some(func4),
                false,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![
                    Value::String("REPLACE".to_string()),
                    Value::String("COPY".to_string()),
                ]),
                Some(func5),
                false,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
                Some(func6),
                false,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![
                    Value::String("eu-west-1".to_string()),
                    Value::String("us-west-1".to_string()),
                ]),
                Some(func7),
                false,
            ),
            // Unsupported value error.
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![
                    Value::String("mybucket/myobjcet".to_string()),
                    Value::Int(7),
                ]),
                None,
                true,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String("AES256".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String("REPLACE".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![Value::String("eu-west-1".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            // Invalid value error.
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![Value::String("mybucket".to_string())]),
                None,
                true,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String("SSE-C".to_string())]),
                None,
                true,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String("DUPLICATE".to_string())]),
                None,
                true,
            ),
        ];

        for (key, values, expected_result, expect_err) in cases {
            let key_cache = key.clone();
            let result = new_string_equals_func(key, values);

            match result {
                Ok(result) => {
                    if let Some(expected_result) = expected_result {
                        assert_eq!(
                            result.to_string(),
                            expected_result.to_string(),
                            "key: '{}', expected: {}, got: {}",
                            key_cache,
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

    #[test]
    fn test_new_string_not_equals_func() -> anyhow::Result<()> {
        let func1 = new_string_not_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func2 = new_string_not_equals_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/myobject".to_string()),
                Value::String("yourbucket/myobject".to_string()),
            ]),
        )?;

        let func3 = new_string_not_equals_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func4 = new_string_not_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let func5 = new_string_not_equals_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPLACE".to_string()),
                Value::String("COPY".to_string()),
            ]),
        )?;

        let func6 = new_string_not_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
        )?;

        let func7 = new_string_not_equals_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-1".to_string()),
                Value::String("us-west-1".to_string()),
            ]),
        )?;

        let cases = [
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
                Some(func1),
                false,
            ),
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![
                    Value::String("mybucket/myobject".to_string()),
                    Value::String("yourbucket/myobject".to_string()),
                ]),
                Some(func2),
                false,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String("AES256".to_string())]),
                Some(func3),
                false,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String("REPLACE".to_string())]),
                Some(func4),
                false,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![
                    Value::String("REPLACE".to_string()),
                    Value::String("COPY".to_string()),
                ]),
                Some(func5),
                false,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![Value::String("eu-west-1".to_string())]),
                Some(func6),
                false,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![
                    Value::String("eu-west-1".to_string()),
                    Value::String("us-west-1".to_string()),
                ]),
                Some(func7),
                false,
            ),
            // Unsupported value error.
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![
                    Value::String("mybucket/myobjcet".to_string()),
                    Value::Int(7),
                ]),
                None,
                true,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String("AES256".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String("REPLACE".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![Value::String("eu-west-1".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            // Invalid value error.
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![Value::String("mybucket".to_string())]),
                None,
                true,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String("SSE-C".to_string())]),
                None,
                true,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String("DUPLICATE".to_string())]),
                None,
                true,
            ),
        ];

        for (key, values, expected_result, expect_err) in cases {
            let key_cache = key.clone();
            let result = new_string_not_equals_func(key, values);

            match result {
                Ok(result) => {
                    if let Some(expected_result) = expected_result {
                        assert_eq!(
                            result.to_string(),
                            expected_result.to_string(),
                            "key: '{}', expected: {}, got: {}",
                            key_cache,
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
