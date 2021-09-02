use std::collections::HashMap;
use std::fmt;

use anyhow::bail;

use super::super::Valid;
use super::*;

// Null condition function. It checks whether Key is not present in given
// values or not.
// For example,
//   1. if Key = S3XAmzCopySource and Value = true, at evaluate() it returns whether
//      S3XAmzCopySource is NOT in given value map or not.
//   2. if Key = S3XAmzCopySource and Value = false, at evaluate() it returns whether
//      S3XAmzCopySource is in given value map or not.
// https://docs.aws.amazon.com/IAM/latest/UserGuide/reference_policies_elements_condition_operators.html#Conditions_Null
#[derive(Clone, Debug)]
pub(super) struct NullFunc<'a> {
    key: Key<'a>,
    value: bool,
}

impl<'a> fmt::Display for NullFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", NULL, self.key, self.value)
    }
}

impl<'a> Function for NullFunc<'a> {
    // Evaluates to check whether Key is present in given values or not.
    // Depending on condition boolean value, this function returns true or false.
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        let mut v = values.get(&canonical_key(self.key.name()));
        if v.is_none() {
            v = values.get(self.key.name());
        }
        match v {
            Some(v) => {
                if self.value {
                    v.is_empty()
                } else {
                    !v.is_empty()
                }
            }
            None => self.value,
        }
    }

    fn key(&self) -> Key<'a> {
        self.key.clone()
    }

    fn name(&self) -> Name<'a> {
        NULL
    }

    fn to_map(&self) -> HashMap<Key<'a>, ValueSet> {
        let mut map = HashMap::new();
        if !self.key.is_valid() {
            return map;
        }
        map.insert(
            self.key.clone(),
            ValueSet::new(vec![Value::Bool(self.value)]),
        );
        map
    }
}

pub(in super::super) fn new_null_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    if values.len() != 1 {
        bail!("only one value is allowed for {} condition", NULL);
    }
    let value = match values.0.into_iter().next().unwrap() {
        Value::Bool(v) => v,
        Value::String(s) => crate::utils::parse_bool(&s).map_err(|_| {
            anyhow::anyhow!("value must be a boolean string for {} condition", NULL)
        })?,
        _ => {
            bail!("value must be a boolean for {} condition", NULL);
        }
    };

    Ok(Box::new(NullFunc { key, value }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_func_evaluate() -> anyhow::Result<()> {
        let case1_fn = new_null_func(S3_PREFIX, ValueSet::new(vec![Value::Bool(true)]))?;
        let case2_fn = new_null_func(S3_PREFIX, ValueSet::new(vec![Value::Bool(false)]))?;

        let cases = [
            (
                &case1_fn,
                HashMap::from([(String::from("prefix"), vec![String::from("true")])]),
                false,
            ),
            (
                &case1_fn,
                HashMap::from([(String::from("prefix"), vec![String::from("false")])]),
                false,
            ),
            (
                &case1_fn,
                HashMap::from([(String::from("prefix"), vec![String::from("mybucket/foo")])]),
                false,
            ),
            (&case1_fn, HashMap::<String, Vec<String>>::new(), true),
            (
                &case1_fn,
                HashMap::from([(String::from("delimiter"), vec![String::from("/")])]),
                true,
            ),
            (
                &case2_fn,
                HashMap::from([(String::from("prefix"), vec![String::from("true")])]),
                true,
            ),
            (
                &case2_fn,
                HashMap::from([(String::from("prefix"), vec![String::from("false")])]),
                true,
            ),
            (
                &case2_fn,
                HashMap::from([(String::from("prefix"), vec![String::from("mybucket/foo")])]),
                true,
            ),
            (&case2_fn, HashMap::<String, Vec<String>>::new(), false),
            (
                &case2_fn,
                HashMap::from([(String::from("delimiter"), vec![String::from("/")])]),
                false,
            ),
        ];

        for (key, values, expected_result) in cases {
            let result = key.evaluate(&values);
            assert_eq!(
                result, expected_result,
                "key: '{}', values: '{:?}', expected: {}, got: {}",
                key, values, expected_result, result
            );
        }

        Ok(())
    }

    #[test]
    fn test_null_func_key() -> anyhow::Result<()> {
        let case_fn = new_null_func(S3X_AMZ_COPY_SOURCE, ValueSet::new(vec![Value::Bool(true)]))?;

        let cases = [(case_fn, S3X_AMZ_COPY_SOURCE)];

        for (key, expected_result) in cases {
            let result = key.key();
            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result,
            );
        }

        Ok(())
    }

    #[test]
    fn test_null_fn_to_map() -> anyhow::Result<()> {
        let case1_fn = new_null_func(S3_PREFIX, ValueSet::new(vec![Value::Bool(true)]))?;
        let case1_res = HashMap::from([(S3_PREFIX, ValueSet::new(vec![Value::Bool(true)]))]);

        let case2_fn = new_null_func(S3_PREFIX, ValueSet::new(vec![Value::Bool(false)]))?;
        let case2_res = HashMap::from([(S3_PREFIX, ValueSet::new(vec![Value::Bool(false)]))]);

        let cases = [(case1_fn, case1_res), (case2_fn, case2_res)];

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
    fn test_new_null_func() -> anyhow::Result<()> {
        let case1_fn = new_null_func(S3_PREFIX, ValueSet::new(vec![Value::Bool(true)]))?;
        let case2_fn = new_null_func(S3_PREFIX, ValueSet::new(vec![Value::Bool(false)]))?;

        let cases = [
            (
                S3_PREFIX,
                ValueSet::new(vec![Value::Bool(true)]),
                Ok(case1_fn),
            ),
            (
                S3_PREFIX,
                ValueSet::new(vec![Value::String("false".to_string())]),
                Ok(case2_fn),
            ),
            // Multiple values error.
            (
                S3_PREFIX,
                ValueSet::new(vec![Value::Bool(true), Value::String("false".to_string())]),
                Err(anyhow::anyhow!(
                    "only one value is allowed for {} condition",
                    NULL
                )),
            ),
            // Invalid boolean string error.
            (
                S3_PREFIX,
                ValueSet::new(vec![Value::String("foo".to_string())]),
                Err(anyhow::anyhow!(
                    "value must be a boolean string for {} condition",
                    NULL
                )),
            ),
            // Invalid value error.
            (
                S3_PREFIX,
                ValueSet::new(vec![Value::Int(7)]),
                Err(anyhow::anyhow!(
                    "value must be a boolean for {} condition",
                    NULL
                )),
            ),
        ];

        for (key, values, expected_result) in cases {
            let key_cache = key.clone();
            let result = new_null_func(key, values);

            match result {
                Ok(result) => {
                    if let Ok(expected_result) = expected_result {
                        assert_eq!(
                            result.to_string(),
                            expected_result.to_string(),
                            "key: '{:?}', expected: {}, got {}",
                            key_cache,
                            expected_result,
                            result,
                        );
                    } else {
                        bail!("not expected an error");
                    }
                }
                Err(err) => {
                    if let Err(expected_result) = expected_result {
                        assert_eq!(
                            err.to_string(),
                            expected_result.to_string(),
                            "key: '{}', expected: {}, got: {}",
                            key_cache,
                            expected_result,
                            err
                        );
                    } else {
                        bail!("expected an error");
                    };
                }
            }
        }

        Ok(())
    }
}
