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
#[derive(Clone)]
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
            None => false,
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

pub(super) fn new_null_func(key: Key, values: ValueSet) -> anyhow::Result<Box<dyn Function + '_>> {
    if values.len() != 1 {
        bail!("only one value is allowed for {} condition", NULL);
    }
    let value = match values.0.into_iter().next().unwrap() {
        Value::Bool(v) => v,
        Value::String(s) => parse_bool(&s).map_err(|_| {
            anyhow::anyhow!("value must be a boolean string for {} condition", NULL)
        })?,
        _ => {
            bail!("value must be a boolean for {} condition", NULL);
        }
    };

    Ok(Box::new(NullFunc { key, value }))
}
