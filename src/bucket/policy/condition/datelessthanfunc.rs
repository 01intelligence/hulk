use std::collections::HashMap;
use std::fmt;

use super::super::Valid;
use super::*;
use crate::utils;
use crate::utils::DateTimeFormatExt;

// String equals function. It checks whether value by Key in given
// values map is in condition values.
// For example,
//   - if values = ["mybucket/foo"], at evaluate() it returns whether string
//     in value map for Key is in values.
#[derive(Clone)]
pub(super) struct DateLessThanFunc<'a> {
    key: Key<'a>,
    value: utils::DateTime,
}

impl<'a> fmt::Display for DateLessThanFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            DATE_LESS_THAN,
            self.key,
            self.value.rfc3339()
        )
    }
}

impl<'a> Function for DateLessThanFunc<'a> {
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
                match utils::DateTime::from_rfc3339(&v[0]) {
                    Ok(v) => v < self.value,
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
        DATE_LESS_THAN
    }

    fn to_map(&self) -> HashMap<Key<'a>, ValueSet> {
        let mut map = HashMap::new();
        if !self.key.is_valid() {
            return map;
        }
        map.insert(
            self.key.clone(),
            ValueSet::new(vec![Value::String(self.value.rfc3339())]),
        );
        map
    }
}

// String not equals function. It checks whether value by Key in
// given values is NOT in condition values.
// For example,
//   - if values = ["mybucket/foo"], at evaluate() it returns whether string
//     in value map for Key is NOT in values.
#[derive(Clone)]
pub(super) struct DateLessThanEqualsFunc<'a>(DateLessThanFunc<'a>);

impl<'a> fmt::Display for DateLessThanEqualsFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            DATE_LESS_THAN_EQUALS,
            self.0.key,
            self.0.value.rfc3339()
        )
    }
}

impl<'a> Function for DateLessThanEqualsFunc<'a> {
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
                match utils::DateTime::from_rfc3339(&v[0]) {
                    Ok(v) => v <= self.0.value,
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
        DATE_LESS_THAN_EQUALS
    }

    fn to_map(&self) -> HashMap<Key<'_>, ValueSet> {
        self.0.to_map()
    }
}

pub(super) fn new_date_less_than_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    Ok(Box::new(DateLessThanFunc {
        key,
        value: value_to_date_time(DATE_LESS_THAN, values)?,
    }))
}

pub(super) fn new_date_less_than_equals_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    Ok(Box::new(DateLessThanEqualsFunc(DateLessThanFunc {
        key,
        value: value_to_date_time(DATE_LESS_THAN_EQUALS, values)?,
    })))
}
