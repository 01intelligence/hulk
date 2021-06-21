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
pub(super) struct NumericGreaterThanFunc<'a> {
    key: Key<'a>,
    value: isize,
}

impl<'a> fmt::Display for NumericGreaterThanFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", NUMERIC_GREATER_THAN, self.key, self.value)
    }
}

impl<'a> Function for NumericGreaterThanFunc<'a> {
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
                    Ok(v) => v > self.value,
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
        NUMERIC_GREATER_THAN
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
pub(super) struct NumericGreaterThanEqualsFunc<'a>(NumericGreaterThanFunc<'a>);

impl<'a> fmt::Display for NumericGreaterThanEqualsFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            NUMERIC_GREATER_THAN_EQUALS, self.0.key, self.0.value
        )
    }
}

impl<'a> Function for NumericGreaterThanEqualsFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        let mut v = values.get(&canonical_key(self.0.key.name()));
        if v.is_none() {
            v = values.get(self.0.key.name());
        }
        match v {
            Some(v) => {
                if v.is_empty() {
                    return false;
                }
                match v[0].parse::<isize>() {
                    Ok(v) => v >= self.0.value,
                    Err(_) => false,
                }
            }
            None => false,
        }
    }

    fn key(&self) -> Key<'_> {
        self.0.key()
    }

    fn name(&self) -> Name<'_> {
        NUMERIC_GREATER_THAN_EQUALS
    }

    fn to_map(&self) -> HashMap<Key<'_>, ValueSet> {
        self.0.to_map()
    }
}

pub(super) fn new_numeric_greater_than_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    Ok(Box::new(NumericGreaterThanFunc {
        key,
        value: value_to_int(NUMERIC_GREATER_THAN, values)?,
    }))
}

pub(super) fn new_numeric_greater_than_equals_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    Ok(Box::new(NumericGreaterThanEqualsFunc(
        NumericGreaterThanFunc {
            key,
            value: value_to_int(NUMERIC_GREATER_THAN_EQUALS, values)?,
        },
    )))
}
