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
        Value::String(s) => Value::Bool(parse_bool(&s).map_err(|_| {
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

pub(super) fn parse_bool(s: &str) -> anyhow::Result<bool> {
    match s {
        "1" | "t" | "T" | "true" | "TRUE" | "True" => Ok(true),
        "0" | "f" | "F" | "false" | "FALSE" | "False" => Ok(false),
        _ => Err(anyhow::anyhow!("provided string was not a boolean string")),
    }
}
