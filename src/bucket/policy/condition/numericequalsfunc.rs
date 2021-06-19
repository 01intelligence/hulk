use std::collections::HashMap;
use std::fmt;

use anyhow::bail;
use validator::HasLen;

use super::super::Valid;
use super::*;

// String equals function. It checks whether value by Key in given
// values map is in condition values.
// For example,
//   - if values = ["mybucket/foo"], at evaluate() it returns whether string
//     in value map for Key is in values.
pub(super) struct NumericEqualsFunc<'a> {
    key: Key<'a>,
    value: isize,
}

impl<'a> fmt::Display for NumericEqualsFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", NUMERIC_EQUALS, self.key, self.value)
    }
}

impl<'a> Function for NumericEqualsFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        let mut v = values.get(&canonical_key(self.key.name()));
        if v.is_none() {
            v = values.get(self.key.name());
        }
        match v {
            Some(v) => {
                if v.is_empty() {
                    return false;
                }
                match v[0].parse::<isize>() {
                    Ok(v) => self.value == v,
                    Err(_) => false,
                }
            }
            None => false,
        }
    }

    fn key(&self) -> Key<'a> {
        self.key.clone()
    }

    fn name(&self) -> Name<'a> {
        NUMERIC_EQUALS
    }

    fn to_map(&self) -> HashMap<Key<'a>, ValueSet> {
        let mut map = HashMap::new();
        if !self.key.is_valid() {
            return map;
        }
        map.insert(
            self.key.clone(),
            ValueSet::new(vec![Value::Int(self.value)]),
        );
        map
    }
}

// String not equals function. It checks whether value by Key in
// given values is NOT in condition values.
// For example,
//   - if values = ["mybucket/foo"], at evaluate() it returns whether string
//     in value map for Key is NOT in values.
pub(super) struct NumericNotEqualsFunc<'a>(NumericEqualsFunc<'a>);

impl<'a> fmt::Display for NumericNotEqualsFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", NUMERIC_NOT_EQUALS, self.0.key, self.0.value)
    }
}

impl<'a> Function for NumericNotEqualsFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        !self.0.evaluate(values)
    }

    fn key(&self) -> Key<'_> {
        self.0.key()
    }

    fn name(&self) -> Name<'_> {
        NUMERIC_NOT_EQUALS
    }

    fn to_map(&self) -> HashMap<Key<'_>, ValueSet> {
        self.0.to_map()
    }
}

pub(super) fn new_numeric_equals_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    Ok(Box::new(NumericEqualsFunc {
        key,
        value: value_to_int(NUMERIC_EQUALS, values)?,
    }))
}

pub(super) fn new_numeric_not_equals_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    Ok(Box::new(NumericNotEqualsFunc(NumericEqualsFunc {
        key,
        value: value_to_int(NUMERIC_NOT_EQUALS, values)?,
    })))
}

fn value_to_int(name: Name, values: ValueSet) -> anyhow::Result<isize> {
    if values.len() != 1 {
        bail!("only one value is allowed for {} condition", name);
    }
    match values.0.into_iter().next().unwrap() {
        Value::Int(v) => Ok(v),
        Value::String(s) => Ok(s
            .parse::<isize>()
            .map_err(|_| anyhow::anyhow!("value must be a int string for {} condition", name))?),
        _ => {
            bail!("value must be a int for {} condition", name);
        }
    }
}
