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
pub(super) struct StringEqualsFunc<'a> {
    key: Key<'a>,
    values: StringSet,
}

impl<'a> fmt::Display for StringEqualsFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", STRING_EQUALS, self.key, self.values)
    }
}

impl<'a> Function for StringEqualsFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        let mut v = values.get(&canonical_key(self.key.name()));
        if v.is_none() {
            v = values.get(self.key.name());
        }
        match v {
            Some(v) => {
                let fvalues = self.values.apply_fn(subst_func_from_values(values.clone()));
                !fvalues
                    .intersection(&StringSet::from_vec(v.clone()))
                    .is_empty()
            }
            None => false,
        }
    }

    fn key(&self) -> Key<'a> {
        self.key.clone()
    }

    fn name(&self) -> Name<'a> {
        STRING_EQUALS
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
pub(super) struct StringNotEqualsFunc<'a>(StringEqualsFunc<'a>);

impl<'a> fmt::Display for StringNotEqualsFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", STRING_NOT_EQUALS, self.0.key, self.0.values)
    }
}

impl<'a> Function for StringNotEqualsFunc<'a> {
    fn evaluate(&self, values: &HashMap<String, Vec<String>>) -> bool {
        !self.0.evaluate(values)
    }

    fn key(&self) -> Key<'_> {
        self.0.key()
    }

    fn name(&self) -> Name<'_> {
        STRING_NOT_EQUALS
    }

    fn to_map(&self) -> HashMap<Key<'_>, ValueSet> {
        self.0.to_map()
    }
}

pub(super) fn new_string_equals_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    let value_strs = values_to_string_slice(STRING_EQUALS, values)?;
    let set = StringSet::from_vec(value_strs);
    validate_string_equals_values(STRING_EQUALS, key.clone(), &set)?;
    Ok(Box::new(StringEqualsFunc { key, values: set }))
}

pub(super) fn new_string_not_equals_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    let value_strs = values_to_string_slice(STRING_NOT_EQUALS, values)?;
    let set = StringSet::from_vec(value_strs);
    validate_string_equals_values(STRING_NOT_EQUALS, key.clone(), &set)?;
    Ok(Box::new(StringNotEqualsFunc(StringEqualsFunc {
        key,
        values: set,
    })))
}

pub(super) fn validate_string_equals_values(name: Name, key: Key, values: &StringSet) -> anyhow::Result<()> {
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
            S3X_AMZ_SERVER_SIDE_ENCRYPTION | S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM => {
                if s != "AES256" {
                    bail!(
                        "invalid value '{}' for '{}' for {} condition",
                        s,
                        S3X_AMZ_SERVER_SIDE_ENCRYPTION,
                        name
                    );
                }
            }
            S3X_AMZ_METADATA_DIRECTIVE => {
                if s != "COPY" && s != "REPLACE" {
                    bail!(
                        "invalid value '{}' for '{}' for {} condition",
                        s,
                        S3X_AMZ_METADATA_DIRECTIVE,
                        name
                    );
                }
            }
            S3X_AMZ_CONTENT_SHA256 => {
                if s.is_empty() {
                    bail!(
                        "invalid empty value for '{}' for {} condition",
                        S3X_AMZ_CONTENT_SHA256,
                        name
                    );
                }
            }
            _ => {}
        }
    }
    Ok(())
}
