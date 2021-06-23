use std::collections::HashMap;
use std::fmt;

use anyhow::bail;

use super::super::Valid;
use super::*;
use crate::strset::StringSet;

#[derive(Clone)]
pub(super) struct BinaryEqualsFunc<'a> {
    key: Key<'a>,
    values: StringSet,
}

impl<'a> fmt::Display for BinaryEqualsFunc<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", BINARY_EQUALS, self.key, self.values)
    }
}

impl<'a> Function for BinaryEqualsFunc<'a> {
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
        BINARY_EQUALS
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
                .map(|&v| Value::String(base64::encode(v)))
                .collect(),
        );
        map.insert(self.key.clone(), values);
        map
    }
}

pub(super) fn new_binary_equals_func(
    key: Key,
    values: ValueSet,
) -> anyhow::Result<Box<dyn Function + '_>> {
    let value_strs = values_to_string_slice(BINARY_EQUALS, values)?;
    let mut set = StringSet::from_vec(value_strs);
    validate_binary_equals_values(BINARY_EQUALS, key.clone(), &mut set)?;
    Ok(Box::new(BinaryEqualsFunc { key, values: set }))
}

fn validate_binary_equals_values(
    name: Name,
    key: Key,
    values: &mut StringSet,
) -> anyhow::Result<()> {
    for s in values.to_vec() {
        let s_bytes = base64::decode(&s)?;
        values.remove(&s);
        let s = std::str::from_utf8(&s_bytes)?;

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

        values.add(s.to_owned());
    }
    Ok(())
}
