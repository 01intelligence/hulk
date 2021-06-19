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
