use std::collections::HashSet;
use std::fmt;

use anyhow::bail;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{self, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Debug)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Int(isize),
    Bool(bool),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Unique list of values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueSet(pub HashSet<Value>);

impl ValueSet {
    pub fn new(values: Vec<Value>) -> ValueSet {
        let mut set = ValueSet(HashSet::new());
        set.0.extend(values.into_iter());
        set
    }

    pub fn insert(&mut self, v: Value) -> bool {
        self.0.insert(v)
    }

    pub fn contains(&self, v: &Value) -> bool {
        self.0.contains(v)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl super::super::ToVec<Value> for ValueSet {
    fn to_vec(&self) -> Vec<Value> {
        self.0.iter().cloned().collect()
    }
}

impl<'a> ser::Serialize for ValueSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if self.0.is_empty() {
            return Err(S::Error::custom("empty value set"));
        }
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for v in &self.0 {
            seq.serialize_element(v)?;
        }
        seq.end()
    }
}

impl<'de> de::Deserialize<'de> for ValueSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueSetVisitor;
        impl<'de> Visitor<'de> for ValueSetVisitor {
            type Value = ValueSet;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a condition value set")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                use serde::de::Error;
                let mut set = ValueSet::new(vec![]);
                while let Some(v) = seq.next_element()? {
                    if set.contains(&v) {
                        return Err(A::Error::custom(format!("duplicate value found '{}'", v)));
                    }
                    set.insert(v);
                }
                if set.is_empty() {
                    return Err(A::Error::custom("empty value set"));
                }
                Ok(set)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ValueSet::new(vec![Value::String(v.to_string())]))
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ValueSet::new(vec![Value::Bool(v)]))
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ValueSet::new(vec![Value::Int(v as isize)]))
            }
        }

        deserializer.deserialize_any(ValueSetVisitor)
    }
}

pub(super) fn values_to_string_slice(name: Name, values: ValueSet) -> anyhow::Result<Vec<String>> {
    let mut ss = Vec::new();
    for value in values.0 {
        if let Value::String(s) = value {
            ss.push(s);
        } else {
            bail!("value must be a string for {} condition", name);
        }
    }
    Ok(ss)
}

// Splits an incoming path into bucket and object components.
pub(super) fn path_to_bucket_and_object(path: &str) -> (&str, &str) {
    // Skip the first element if it is '/', split the rest.
    let parts: Vec<&str> = path.trim_start_matches('/').splitn(2, '/').collect();

    match parts.len() {
        2 => (parts[0], parts[1]),
        1 => (parts[0], ""),
        _ => ("", ""),
    }
}

#[cfg(test)]
mod tests {
    use std::any::type_name;

    use super::*;
    use crate::utils::assert::*;

    #[test]
    fn test_value_get_bool() {
        let cases = [
            (Value::Bool(true), true, false),
            (Value::Int(7), false, true),
            (Value::String("foo".to_string()), false, true),
        ];

        for (key, expected_result, expect_err) in cases {
            match key {
                Value::Bool(v) => {
                    assert_eq!(
                        v, expected_result,
                        "key: '{}', expected: {}, got: {}",
                        key, expected_result, v
                    )
                }
                Value::String(_) | Value::Int(_) => {
                    assert!(expect_err, "expect an error")
                }
            }
        }
    }

    #[test]
    fn test_value_get_int() {
        let cases = [
            (Value::Int(7), 7, false),
            (Value::Bool(true), 0, true),
            (Value::String("foo".to_string()), 0, true),
        ];

        for (key, expected_result, expect_err) in cases {
            match key {
                Value::Int(v) => {
                    assert_eq!(
                        v, expected_result,
                        "key: '{}', expected: {}, got: {}",
                        key, expected_result, v
                    )
                }
                Value::String(_) | Value::Bool(_) => {
                    assert!(expect_err, "expect an error")
                }
            }
        }
    }

    #[test]
    fn test_value_get_string() {
        let cases = [
            (Value::String("foo".to_string()), "foo", false),
            (Value::Int(7), "", true),
            (Value::Bool(true), "", true),
        ];

        for (key, expected_result, expect_err) in cases {
            let key_cache = key.clone();
            match key {
                Value::String(v) => {
                    assert_eq!(
                        v, expected_result,
                        "key: '{}', expected: {}, got: {}",
                        key_cache, expected_result, v
                    );
                }
                Value::Bool(_) | Value::Int(_) => {
                    assert!(expect_err, "expect an error");
                }
            }
        }
    }

    #[test]
    fn test_value_get_type() {
        fn type_of<T>(_: &T) -> &'static str {
            type_name::<T>()
        }

        let cases = [
            (Value::Bool(true), "bool"),
            (Value::Int(7), "isize"),
            (Value::String("foo".to_string()), "alloc::string::String"),
        ];

        for (key, expected_result) in cases {
            let key_cache = key.clone();
            match key {
                Value::String(v) => {
                    let result = type_of::<String>(&v);
                    assert_eq!(
                        result, expected_result,
                        "key: '{}', expected: {}, got: {}",
                        key_cache, expected_result, result
                    );
                }
                Value::Int(v) => {
                    let result = type_of::<isize>(&v);
                    assert_eq!(
                        result, expected_result,
                        "key: '{}', expected: {}, got: {}",
                        key_cache, expected_result, result
                    );
                }
                Value::Bool(v) => {
                    let result = type_of::<bool>(&v);
                    assert_eq!(
                        result, expected_result,
                        "key: '{}', expected: {}, got: {}",
                        key_cache, expected_result, result
                    );
                }
            }
        }
    }

    #[test]
    fn test_value_store_bool() {
        let cases = [(false, Value::Bool(false)), (true, Value::Bool(true))];

        for (key, expected_result) in cases {
            let result = Value::Bool(key);

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }
    }

    #[test]
    fn test_value_store_int() {
        let cases = [(0isize, Value::Int(0)), (7isize, Value::Int(7))];

        for (key, expected_result) in cases {
            let result = Value::Int(key);

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }
    }

    #[test]
    fn test_value_store_string() {
        let cases = [
            ("", Value::String("".to_string())),
            ("foo", Value::String("foo".to_string())),
        ];

        for (key, expected_result) in cases {
            let result = Value::String(key.to_string());

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }
    }

    #[test]
    fn test_value_string() {
        let cases = [
            (Value::Bool(true), "Bool(true)"),
            (Value::Int(7), "Int(7)"),
            (Value::String("foo".to_string()), r#"String("foo")"#),
        ];

        for (key, expected_result) in cases {
            let result = key.to_string();

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }
    }

    #[test]
    fn test_value_serialize_json() {
        let cases = [
            (Value::Bool(true), "true"),
            (Value::Int(7), "7"),
            (Value::String("foo".to_string()), r#""foo""#),
        ];

        for (key, expected_result) in cases {
            let result = assert_ok!(serde_json::to_string(&key));

            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }
    }

    #[test]
    fn test_value_deserialize_json() {
        let cases = [
            ("true", Value::Bool(true)),
            ("7", Value::Int(7)),
            (r#""foo""#, Value::String(String::from("foo"))),
        ];

        for (key, expected_result) in cases {
            let result = assert_ok!(serde_json::from_str::<Value>(key));
            assert_eq!(
                result, expected_result,
                "key: '{}', expected: {}, got: {}",
                key, expected_result, result
            );
        }
    }
}
