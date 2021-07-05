use std::collections::HashSet;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct StringSet(HashSet<String>);

impl StringSet {
    pub fn new() -> StringSet {
        StringSet(HashSet::new())
    }

    pub fn from_slice(ss: &[&str]) -> StringSet {
        StringSet(ss.iter().map(|&s| s.into()).collect())
    }

    pub fn from_vec(ss: Vec<String>) -> StringSet {
        StringSet(ss.into_iter().collect())
    }

    pub fn as_slice(&self) -> Vec<&str> {
        let mut ss: Vec<&str> = self.0.iter().map(|s| s as &str).collect();
        ss.sort_unstable();
        ss
    }

    pub fn to_vec(&self) -> Vec<String> {
        let mut ss: Vec<String> = self.0.iter().cloned().collect();
        ss.sort_unstable();
        ss
    }

    pub fn iter(&self) -> std::collections::hash_set::Iter<'_, String> {
        self.0.iter()
    }

    pub fn into_iter(self) -> std::collections::hash_set::IntoIter<String> {
        self.0.into_iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn add(&mut self, s: String) {
        self.0.insert(s);
    }

    pub fn remove(&mut self, s: &str) {
        self.0.remove(s);
    }

    pub fn contains(&self, s: &str) -> bool {
        self.0.contains(s)
    }

    pub fn match_fn<F>(&self, mut match_fn: F) -> StringSet
    where
        F: FnMut(&str) -> bool,
    {
        StringSet(self.0.iter().filter(|&s| match_fn(s)).cloned().collect())
    }

    pub fn apply_fn<F>(&self, apply_fn: F) -> StringSet
    where
        F: Fn(&str) -> String,
    {
        StringSet(self.0.iter().map(|s| apply_fn(s)).collect())
    }

    pub fn intersection(&self, other: &StringSet) -> StringSet {
        StringSet(self.0.intersection(&other.0).cloned().collect())
    }

    pub fn difference(&self, other: &StringSet) -> StringSet {
        StringSet(self.0.difference(&other.0).cloned().collect())
    }

    pub fn union(&self, other: &StringSet) -> StringSet {
        StringSet(self.0.union(&other.0).cloned().collect())
    }
}

impl Default for StringSet {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for StringSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let slice = self.as_slice();
        write!(f, "[")?;
        write!(f, "{}", slice.join(","))?;
        write!(f, "]")
    }
}

impl std::iter::FromIterator<String> for StringSet {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut ss = StringSet::new();
        for s in iter {
            ss.add(s);
        }
        ss
    }
}

impl<'a> Serialize for StringSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for v in &self.0 {
            seq.serialize_element(v)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for StringSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringSetVisitor;
        impl<'de> Visitor<'de> for StringSetVisitor {
            type Value = StringSet;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string array or a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(crate::string_set!(v.to_owned()))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                use serde::de::Error;
                let mut set = StringSet::default();
                while let Some(v) = seq.next_element()? {
                    set.add(v);
                }
                Ok(set)
            }
        }

        deserializer.deserialize_any(StringSetVisitor)
    }
}

#[macro_export]
macro_rules! string_set {
    ($($e:expr),*) => {{
        let mut set = StringSet::default();
        $(
            set.add($e);
        )*
        set
    }};
}
