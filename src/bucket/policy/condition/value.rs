use std::collections::HashSet;
use std::fmt;

use anyhow::bail;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{self, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Debug)]
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
        }

        deserializer.deserialize_seq(ValueSetVisitor)
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
