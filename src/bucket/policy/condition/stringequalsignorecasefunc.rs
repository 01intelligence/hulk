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
pub(super) struct StringEqualsIgnoreCaseFunc<'a> {
    key: Key<'a>,
    values: StringSet,
}

impl<'a> fmt::Display for StringEqualsIgnoreCaseFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            STRING_EQUALS_IGNORE_CASE, self.key, self.values
        )
    }
}

impl<'a> Function for StringEqualsIgnoreCaseFunc<'a> {
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
                        .match_fn(|ss| unicase::UniCase::new(ss) == unicase::UniCase::new(s))
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
        STRING_EQUALS_IGNORE_CASE
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
pub(super) struct StringNotEqualsIgnoreCaseFunc<'a>(StringEqualsIgnoreCaseFunc<'a>);

impl<'a> fmt::Display for StringNotEqualsIgnoreCaseFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            STRING_NOT_EQUALS_IGNORE_CASE, self.0.key, self.0.values
        )
    }
}

impl<'a> Function for StringNotEqualsIgnoreCaseFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        !self.0.evaluate(values)
    }

    fn key(&self) -> Key<'_> {
        self.0.key()
    }

    fn name(&self) -> Name<'_> {
        STRING_NOT_EQUALS_IGNORE_CASE
    }

    fn to_map(&self) -> HashMap<Key<'_>, ValueSet> {
        self.0.to_map()
    }
}

pub(super) fn new_string_equals_ignore_case_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    let value_strs = values_to_string_slice(STRING_EQUALS_IGNORE_CASE, values)?;
    let set = StringSet::from_vec(value_strs);
    validate_string_equals_values(STRING_EQUALS_IGNORE_CASE, key.clone(), &set)?;
    Ok(Box::new(StringEqualsIgnoreCaseFunc { key, values: set }))
}

pub(super) fn new_string_not_equals_ignore_case_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    let value_strs = values_to_string_slice(STRING_NOT_EQUALS_IGNORE_CASE, values)?;
    let set = StringSet::from_vec(value_strs);
    validate_string_equals_values(STRING_NOT_EQUALS_IGNORE_CASE, key.clone(), &set)?;
    Ok(Box::new(StringNotEqualsIgnoreCaseFunc(
        StringEqualsIgnoreCaseFunc { key, values: set },
    )))
}
