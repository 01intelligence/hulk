use std::collections::HashMap;
use std::fmt;

use anyhow::bail;

use super::super::Valid;
use super::*;

// Bool condition function. It checks whether Key is true or false.
// https://docs.aws.amazon.com/IAM/latest/UserGuide/reference_policies_elements_condition_operators.html#Conditions_Boolean
#[derive(Clone)]
pub(super) struct BooleanFunc<'a> {
    key: Key<'a>,
    value: String,
}

impl<'a> fmt::Display for BooleanFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", BOOLEAN, self.key, self.value)
    }
}

impl<'a> Function for BooleanFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        let mut v = values.get(&super::canonical_key(self.key.name()));
        if v.is_none() {
            v = values.get(self.key.name());
        }
        match v {
            Some(v) => self.value == v[0],
            None => false,
        }
    }

    fn key(&self) -> Key<'a> {
        self.key.clone()
    }

    fn name(&self) -> Name {
        BOOLEAN
    }

    fn to_map(&self) -> HashMap<Key<'a>, ValueSet> {
        let mut map = HashMap::new();
        if self.key.is_valid() {
            map.insert(
                self.key.clone(),
                ValueSet::new(vec![Value::String(self.value.clone())]),
            );
        }
        map
    }
}

pub(super) fn new_boolean_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    if key != AWS_SECURE_TRANSPORT {
        bail!(
            "only {} key is allowed for {} condition",
            AWS_SECURE_TRANSPORT,
            BOOLEAN
        );
    }
    if values.len() != 1 {
        bail!("only one value is allowed for {} condition", BOOLEAN);
    }
    let value = match values.0.into_iter().next().unwrap() {
        Value::Bool(v) => Value::Bool(v),
        Value::String(s) => Value::Bool(crate::utils::parse_bool(&s).map_err(|_| {
            anyhow::anyhow!("value must be a boolean string for {} condition", BOOLEAN)
        })?),
        _ => {
            bail!("value must be a boolean for {} condition", BOOLEAN);
        }
    };

    Ok(Box::new(BooleanFunc {
        key,
        value: value.to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boolean_func_evaluate() -> anyhow::Result<()> {
        let func1 = new_boolean_func(AWS_SECURE_TRANSPORT, ValueSet::new(vec![Value::Bool(true)]))?;

        let func2 = new_boolean_func(
            AWS_SECURE_TRANSPORT,
            ValueSet::new(vec![Value::Bool(false)]),
        )?;

        let cases = [
            (
                func1,
                HashMap::from([("SecureTransport".to_string(), vec!["true".to_string()])]),
                true,
            ),
            (
                func2,
                HashMap::from([("SecureTransport".to_string(), vec!["false".to_string()])]),
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
    fn test_boolean_func_key() -> anyhow::Result<()> {
        let func = new_boolean_func(AWS_SECURE_TRANSPORT, ValueSet::new(vec![Value::Bool(true)]))?;

        let cases = [(func, AWS_SECURE_TRANSPORT)];

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
    fn test_boolean_func_to_map() -> anyhow::Result<()> {
        let func1 = new_boolean_func(AWS_SECURE_TRANSPORT, ValueSet::new(vec![Value::Bool(true)]))?;

        let res1 = HashMap::from([(
            AWS_SECURE_TRANSPORT,
            ValueSet::new(vec![Value::String("true".to_string())]),
        )]);

        let func2 = new_boolean_func(
            AWS_SECURE_TRANSPORT,
            ValueSet::new(vec![Value::Bool(false)]),
        )?;

        let res2 = HashMap::from([(
            AWS_SECURE_TRANSPORT,
            ValueSet::new(vec![Value::String("false".to_string())]),
        )]);

        let cases = [(func1, res1), (func2, res2)];

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
    fn test_new_boolean_func() -> anyhow::Result<()> {
        let func1 = new_boolean_func(AWS_SECURE_TRANSPORT, ValueSet::new(vec![Value::Bool(true)]))?;

        let func2 = new_boolean_func(
            AWS_SECURE_TRANSPORT,
            ValueSet::new(vec![Value::Bool(false)]),
        )?;

        let cases = [
            (
                AWS_SECURE_TRANSPORT,
                ValueSet::new(vec![Value::Bool(true)]),
                Some(func1),
                false,
            ),
            (
                AWS_SECURE_TRANSPORT,
                ValueSet::new(vec![Value::String("false".to_string())]),
                Some(func2),
                false,
            ),
            // Multiple values error.
            (
                AWS_SECURE_TRANSPORT,
                ValueSet::new(vec![
                    Value::String("true".to_string()),
                    Value::String("false".to_string()),
                ]),
                None,
                true,
            ),
            // Invalid boolean string error.
            (
                AWS_SECURE_TRANSPORT,
                ValueSet::new(vec![Value::String("foo".to_string())]),
                None,
                true,
            ),
            // Invalid value error.
            (
                AWS_SECURE_TRANSPORT,
                ValueSet::new(vec![Value::Int(7)]),
                None,
                true,
            ),
        ];

        for (key, values, expected_result, expect_err) in cases {
            let key_cache = key.clone();
            let result = new_boolean_func(key, values);

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
