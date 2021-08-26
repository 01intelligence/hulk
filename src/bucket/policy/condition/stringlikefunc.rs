use std::collections::HashMap;
use std::fmt;

use anyhow::bail;

use super::super::Valid;
use super::*;
use crate::strset::StringSet;

// String like function. It checks whether value by Key in given
// values map is widcard matching in condition values.
// For example,
//   - if values = ["mybucket/foo*"], at evaluate() it returns whether string
//     in value map for Key is wildcard matching in values.
#[derive(Clone)]
pub(super) struct StringLikeFunc<'a> {
    key: Key<'a>,
    values: StringSet,
}

impl<'a> fmt::Display for StringLikeFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", STRING_LIKE, self.key, self.values)
    }
}

impl<'a> Function for StringLikeFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        let mut v = values.get(&canonical_key(self.key.name()));
        if v.is_none() {
            v = values.get(self.key.name());
        }
        match v {
            Some(v) => {
                let fvalues = self.values.apply_fn(subst_func_from_values(values.clone()));
                for s in v {
                    if !fvalues
                        .match_fn(|ss| crate::wildcard::match_wildcard(ss, s))
                        .is_empty()
                    {
                        return true;
                    }
                }
                return false;
            }
            None => false,
        }
    }

    fn key(&self) -> Key<'a> {
        self.key.clone()
    }

    fn name(&self) -> Name<'a> {
        STRING_LIKE
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

// String not like function. It checks whether value by Key in given
// values map is NOT widcard matching in condition values.
// For example,
//   - if values = ["mybucket/foo*"], at evaluate() it returns whether string
//     in value map for Key is NOT wildcard matching in values.
#[derive(Clone)]
pub(super) struct StringNotLikeFunc<'a>(StringLikeFunc<'a>);

impl<'a> fmt::Display for StringNotLikeFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", STRING_NOT_LIKE, self.0.key, self.0.values)
    }
}

impl<'a> Function for StringNotLikeFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        !self.0.evaluate(values)
    }

    fn key(&self) -> Key<'_> {
        self.0.key()
    }

    fn name(&self) -> Name<'_> {
        STRING_NOT_LIKE
    }

    fn to_map(&self) -> HashMap<Key<'_>, ValueSet> {
        self.0.to_map()
    }
}

pub(super) fn new_string_like_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    let value_strs = values_to_string_slice(STRING_LIKE, values)?;
    let set = StringSet::from_vec(value_strs);
    validate_string_like_values(STRING_LIKE, key.clone(), &set)?;
    Ok(Box::new(StringLikeFunc { key, values: set }))
}

pub(super) fn new_string_not_like_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    let value_strs = values_to_string_slice(STRING_NOT_LIKE, values)?;
    let set = StringSet::from_vec(value_strs);
    validate_string_like_values(STRING_NOT_LIKE, key.clone(), &set)?;
    Ok(Box::new(StringNotLikeFunc(StringLikeFunc {
        key,
        values: set,
    })))
}

fn validate_string_like_values(name: Name, key: Key, values: &StringSet) -> anyhow::Result<()> {
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
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_like_func_evaluate() -> anyhow::Result<()> {
        let func1 = new_string_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject*".to_string())]),
        )?;

        let func2 = new_string_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func3 = new_string_like_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES*".to_string())]),
        )?;

        let func4 = new_string_like_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func5 = new_string_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPL*".to_string())]),
        )?;

        let func6 = new_string_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let func7 = new_string_like_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-*".to_string())]),
        )?;

        let func8 = new_string_like_func(
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
                    vec!["mybucket/myobject.png".to_string()],
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
                    "x-amz-copy-source".to_string(),
                    vec!["mybucket/myobject".to_string()],
                )]),
                true,
            ),
            (
                &func2,
                HashMap::from([(
                    "x-amz-copy-source".to_string(),
                    vec!["mybucket/myobject.png".to_string()],
                )]),
                false,
            ),
            (
                &func2,
                HashMap::from([(
                    "x-amz-copy-source".to_string(),
                    vec!["yourbucket/myobject".to_string()],
                )]),
                false,
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
                    "x-amz-server-side-encryption".to_string(),
                    vec!["AES256".to_string()],
                )]),
                true,
            ),
            (
                &func3,
                HashMap::from([(
                    "x-amz-server-side-encryption".to_string(),
                    vec!["AES512".to_string()],
                )]),
                true,
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
                    "x-amz-server-side-encryption".to_string(),
                    vec!["AES256".to_string()],
                )]),
                true,
            ),
            (
                &func4,
                HashMap::from([(
                    "x-amz-server-side-encryption".to_string(),
                    vec!["AES512".to_string()],
                )]),
                false,
            ),
            (&func4, HashMap::new(), false),
            (
                &func4,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                false,
            ),
            (
                &func5,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["REPLACE".to_string()],
                )]),
                true,
            ),
            (
                &func5,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["REPLACE/COPY".to_string()],
                )]),
                true,
            ),
            (
                &func5,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["COPY".to_string()],
                )]),
                false,
            ),
            (&func5, HashMap::new(), false),
            (
                &func5,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                false,
            ),
            (
                &func6,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["REPLACE".to_string()],
                )]),
                true,
            ),
            (
                &func6,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["REPLACE/COPY".to_string()],
                )]),
                false,
            ),
            (
                &func6,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["COPY".to_string()],
                )]),
                false,
            ),
            (&func6, HashMap::new(), false),
            (
                &func6,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                false,
            ),
            (
                &func7,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["eu-west-1".to_string()],
                )]),
                true,
            ),
            (
                &func7,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["eu-west-2".to_string()],
                )]),
                true,
            ),
            (
                &func7,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["us-west-1".to_string()],
                )]),
                false,
            ),
            (&func7, HashMap::new(), false),
            (
                &func7,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                false,
            ),
            (
                &func8,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["eu-west-1".to_string()],
                )]),
                true,
            ),
            (
                &func8,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["eu-west-2".to_string()],
                )]),
                false,
            ),
            (
                &func8,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["us-west-1".to_string()],
                )]),
                false,
            ),
            (&func8, HashMap::new(), false),
            (
                &func8,
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
    fn test_string_like_func_key() -> anyhow::Result<()> {
        let func1 = new_string_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func2 = new_string_like_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func3 = new_string_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let func4 = new_string_like_func(
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
    fn test_string_like_func_top_map() -> anyhow::Result<()> {
        let func1 = new_string_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/*".to_string())]),
        )?;

        let res1 = HashMap::from([(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/*".to_string())]),
        )]);

        let func2 = new_string_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/*".to_string()),
                Value::String("yourbucket/myobject".to_string()),
            ]),
        )?;

        let res2 = HashMap::from([(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/*".to_string()),
                Value::String("yourbucket/myobject".to_string()),
            ]),
        )]);

        let func3 = new_string_like_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES*".to_string())]),
        )?;

        let res3 = HashMap::from([(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES*".to_string())]),
        )]);

        let func4 = new_string_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPL*".to_string())]),
        )?;

        let res4 = HashMap::from([(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPL*".to_string())]),
        )]);

        let func5 = new_string_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPL*".to_string()),
                Value::String("COPY*".to_string()),
            ]),
        )?;

        let res5 = HashMap::from([(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPL*".to_string()),
                Value::String("COPY*".to_string()),
            ]),
        )]);

        let func6 = new_string_like_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-*".to_string())]),
        )?;

        let res6 = HashMap::from([(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-*".to_string())]),
        )]);

        let func7 = new_string_like_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-*".to_string()),
                Value::String("us-west-*".to_string()),
            ]),
        )?;

        let res7 = HashMap::from([(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-*".to_string()),
                Value::String("us-west-*".to_string()),
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
    fn test_string_not_like_func_evaluate() -> anyhow::Result<()> {
        let func1 = new_string_not_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject*".to_string())]),
        )?;

        let func2 = new_string_not_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func3 = new_string_not_like_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES*".to_string())]),
        )?;

        let func4 = new_string_not_like_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func5 = new_string_not_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPL*".to_string())]),
        )?;

        let func6 = new_string_not_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let func7 = new_string_not_like_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-*".to_string())]),
        )?;

        let func8 = new_string_not_like_func(
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
                    vec!["mybucket/myobject.png".to_string()],
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
            (&func1, HashMap::new(), true),
            (
                &func1,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func2,
                HashMap::from([(
                    "x-amz-copy-source".to_string(),
                    vec!["mybucket/myobject".to_string()],
                )]),
                false,
            ),
            (
                &func2,
                HashMap::from([(
                    "x-amz-copy-source".to_string(),
                    vec!["mybucket/myobject.png".to_string()],
                )]),
                true,
            ),
            (
                &func2,
                HashMap::from([(
                    "x-amz-copy-source".to_string(),
                    vec!["yourbucket/myobject".to_string()],
                )]),
                true,
            ),
            (&func2, HashMap::new(), true),
            (
                &func2,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func3,
                HashMap::from([(
                    "x-amz-server-side-encryption".to_string(),
                    vec!["AES256".to_string()],
                )]),
                false,
            ),
            (
                &func3,
                HashMap::from([(
                    "x-amz-server-side-encryption".to_string(),
                    vec!["AES512".to_string()],
                )]),
                false,
            ),
            (&func3, HashMap::new(), true),
            (
                &func3,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func4,
                HashMap::from([(
                    "x-amz-server-side-encryption".to_string(),
                    vec!["AES256".to_string()],
                )]),
                false,
            ),
            (
                &func4,
                HashMap::from([(
                    "x-amz-server-side-encryption".to_string(),
                    vec!["AES512".to_string()],
                )]),
                true,
            ),
            (&func4, HashMap::new(), true),
            (
                &func4,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func5,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["REPLACE".to_string()],
                )]),
                false,
            ),
            (
                &func5,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["REPLACE/COPY".to_string()],
                )]),
                false,
            ),
            (
                &func5,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["COPY".to_string()],
                )]),
                true,
            ),
            (&func5, HashMap::new(), true),
            (
                &func5,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func6,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["REPLACE".to_string()],
                )]),
                false,
            ),
            (
                &func6,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["REPLACE/COPY".to_string()],
                )]),
                true,
            ),
            (
                &func6,
                HashMap::from([(
                    "x-amz-metadata-directive".to_string(),
                    vec!["COPY".to_string()],
                )]),
                true,
            ),
            (&func6, HashMap::new(), true),
            (
                &func6,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func7,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["eu-west-1".to_string()],
                )]),
                false,
            ),
            (
                &func7,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["eu-west-2".to_string()],
                )]),
                false,
            ),
            (
                &func7,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["us-west-1".to_string()],
                )]),
                true,
            ),
            (&func7, HashMap::new(), true),
            (
                &func7,
                HashMap::from([("delimiter".to_string(), vec!["/".to_string()])]),
                true,
            ),
            (
                &func8,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["eu-west-1".to_string()],
                )]),
                false,
            ),
            (
                &func8,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["eu-west-2".to_string()],
                )]),
                true,
            ),
            (
                &func8,
                HashMap::from([(
                    "LocationConstraint".to_string(),
                    vec!["us-west-1".to_string()],
                )]),
                true,
            ),
            (&func8, HashMap::new(), true),
            (
                &func8,
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
    fn test_string_not_like_func_key() -> anyhow::Result<()> {
        let func1 = new_string_not_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/myobject".to_string())]),
        )?;

        let func2 = new_string_not_like_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES256".to_string())]),
        )?;

        let func3 = new_string_not_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPLACE".to_string())]),
        )?;

        let func4 = new_string_not_like_func(
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
    fn test_string_not_like_func_top_map() -> anyhow::Result<()> {
        let func1 = new_string_not_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/*".to_string())]),
        )?;

        let res1 = HashMap::from([(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/*".to_string())]),
        )]);

        let func2 = new_string_not_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/*".to_string()),
                Value::String("yourbucket/myobject".to_string()),
            ]),
        )?;

        let res2 = HashMap::from([(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/*".to_string()),
                Value::String("yourbucket/myobject".to_string()),
            ]),
        )]);

        let func3 = new_string_not_like_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES*".to_string())]),
        )?;

        let res3 = HashMap::from([(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES*".to_string())]),
        )]);

        let func4 = new_string_not_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPL*".to_string())]),
        )?;

        let res4 = HashMap::from([(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPL*".to_string())]),
        )]);

        let func5 = new_string_not_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPL*".to_string()),
                Value::String("COPY*".to_string()),
            ]),
        )?;

        let res5 = HashMap::from([(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPL*".to_string()),
                Value::String("COPY*".to_string()),
            ]),
        )]);

        let func6 = new_string_not_like_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-*".to_string())]),
        )?;

        let res6 = HashMap::from([(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-*".to_string())]),
        )]);

        let func7 = new_string_not_like_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-*".to_string()),
                Value::String("us-west-*".to_string()),
            ]),
        )?;

        let res7 = HashMap::from([(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-*".to_string()),
                Value::String("us-west-*".to_string()),
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
    fn test_new_string_like_func() -> anyhow::Result<()> {
        let func1 = new_string_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/*".to_string())]),
        )?;

        let func2 = new_string_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/*".to_string()),
                Value::String("yourbucket/myobject*".to_string()),
            ]),
        )?;

        let func3 = new_string_like_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES*".to_string())]),
        )?;

        let func4 = new_string_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPL*".to_string())]),
        )?;

        let func5 = new_string_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPL*".to_string()),
                Value::String("COPY*".to_string()),
            ]),
        )?;

        let func6 = new_string_like_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-*".to_string())]),
        )?;

        let func7 = new_string_like_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-*".to_string()),
                Value::String("us-west-*".to_string()),
            ]),
        )?;

        let cases = [
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![Value::String("mybucket/*".to_string())]),
                Some(func1),
                false,
            ),
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![
                    Value::String("mybucket/*".to_string()),
                    Value::String("yourbucket/myobject*".to_string()),
                ]),
                Some(func2),
                false,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String("AES*".to_string())]),
                Some(func3),
                false,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String("REPL*".to_string())]),
                Some(func4),
                false,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![
                    Value::String("REPL*".to_string()),
                    Value::String("COPY*".to_string()),
                ]),
                Some(func5),
                false,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![Value::String("eu-west-*".to_string())]),
                Some(func6),
                false,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![
                    Value::String("eu-west-*".to_string()),
                    Value::String("us-west-*".to_string()),
                ]),
                Some(func7),
                false,
            ),
            // Unsupported value error.
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![Value::String("mybucket/*".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String("AES*".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String("REPL*".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![Value::String("eu-west-*".to_string()), Value::Int(7)]),
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
        ];

        for (key, values, expected_result, expect_err) in cases {
            let result = new_string_like_func(key.clone(), values);

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

    #[test]
    fn test_new_string_not_like_func() -> anyhow::Result<()> {
        let func1 = new_string_not_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![Value::String("mybucket/*".to_string())]),
        )?;

        let func2 = new_string_not_like_func(
            S3X_AMZ_COPY_SOURCE,
            ValueSet::new(vec![
                Value::String("mybucket/*".to_string()),
                Value::String("yourbucket/myobject*".to_string()),
            ]),
        )?;

        let func3 = new_string_not_like_func(
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            ValueSet::new(vec![Value::String("AES*".to_string())]),
        )?;

        let func4 = new_string_not_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![Value::String("REPL*".to_string())]),
        )?;

        let func5 = new_string_not_like_func(
            S3X_AMZ_METADATA_DIRECTIVE,
            ValueSet::new(vec![
                Value::String("REPL*".to_string()),
                Value::String("COPY*".to_string()),
            ]),
        )?;

        let func6 = new_string_not_like_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![Value::String("eu-west-*".to_string())]),
        )?;

        let func7 = new_string_not_like_func(
            S3_LOCATION_CONSTRAINT,
            ValueSet::new(vec![
                Value::String("eu-west-*".to_string()),
                Value::String("us-west-*".to_string()),
            ]),
        )?;

        let cases = [
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![Value::String("mybucket/*".to_string())]),
                Some(func1),
                false,
            ),
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![
                    Value::String("mybucket/*".to_string()),
                    Value::String("yourbucket/myobject*".to_string()),
                ]),
                Some(func2),
                false,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String("AES*".to_string())]),
                Some(func3),
                false,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String("REPL*".to_string())]),
                Some(func4),
                false,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![
                    Value::String("REPL*".to_string()),
                    Value::String("COPY*".to_string()),
                ]),
                Some(func5),
                false,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![Value::String("eu-west-*".to_string())]),
                Some(func6),
                false,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![
                    Value::String("eu-west-*".to_string()),
                    Value::String("us-west-*".to_string()),
                ]),
                Some(func7),
                false,
            ),
            // Unsupported value error.
            (
                S3X_AMZ_COPY_SOURCE,
                ValueSet::new(vec![Value::String("mybucket/*".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                ValueSet::new(vec![Value::String("AES*".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3X_AMZ_METADATA_DIRECTIVE,
                ValueSet::new(vec![Value::String("REPL*".to_string()), Value::Int(7)]),
                None,
                true,
            ),
            (
                S3_LOCATION_CONSTRAINT,
                ValueSet::new(vec![Value::String("eu-west-*".to_string()), Value::Int(7)]),
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
        ];

        for (key, values, expected_result, expect_err) in cases {
            let result = new_string_not_like_func(key.clone(), values);

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
