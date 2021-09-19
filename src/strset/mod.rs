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

    pub fn len(&self) -> usize {
        self.0.len()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::assert::*;

    #[test]
    fn new_string_set() {
        let ss = StringSet::new();
        assert!(ss.is_empty(), "expected: true, got: false");
    }

    #[test]
    fn create_string_set() {
        let ss = string_set!("foo".to_string());
        assert_eq!(
            ss.to_string(),
            "[foo]",
            "expected: {}, got: {}",
            r#"["foo"]"#,
            ss
        );
    }

    #[test]
    fn string_set_add() {
        let cases = [
            ("foo", string_set!("foo".to_string())),
            ("foo", string_set!("foo".to_string())),
            ("bar", string_set!("bar".to_string(), "foo".to_string())),
        ];

        let mut ss = StringSet::new();
        for (value, expected_result) in cases {
            ss.add(value.to_string());

            assert_eq!(
                ss, expected_result,
                "expected: {}, got: {}",
                expected_result, ss
            );
        }
    }

    #[test]
    fn string_set_remove() {
        let cases = [
            ("baz", string_set!("foo".to_string(), "bar".to_string())),
            ("foo", string_set!("bar".to_string())),
            ("foo", string_set!("bar".to_string())),
            ("bar", StringSet::new()),
        ];

        let mut ss = string_set!("foo".to_string(), "bar".to_string());
        for (value, expected_result) in cases {
            ss.remove(value);

            assert_eq!(
                ss, expected_result,
                "expected: {}, got: {}",
                expected_result, ss
            );
        }
    }

    #[test]
    fn string_set_contains() {
        let cases = [("bar", false), ("foo", true), ("Foo", false)];

        let ss = string_set!("foo".to_string());
        for (value, expected_result) in cases {
            let result = ss.contains(value);

            assert_eq!(
                result, expected_result,
                "expected: {}; got: {}",
                expected_result, result
            );
        }
    }

    #[test]
    fn string_set_func_match() {
        let ss = string_set!("foo".to_string(), "bar".to_string());

        let cases: [(Box<dyn FnMut(&str, &str) -> bool>, &str, StringSet); 2] = [
            (
                Box::new(|value: &str, compare_value: &str| {
                    value.eq_ignore_ascii_case(compare_value)
                }),
                "Bar",
                string_set!("bar".to_string()),
            ),
            (
                Box::new(|value: &str, compare_value: &str| compare_value.starts_with(value)),
                "foobar",
                string_set!("foo".to_string()),
            ),
        ];

        for (mut func, value, expected_result) in cases {
            let result = ss.match_fn(|s| func(s, value));

            assert_eq!(
                result, expected_result,
                "expected: {}, got: {}",
                expected_result, result
            );
        }
    }

    #[test]
    fn string_set_apply_func() {
        let ss = string_set!("foo".to_string(), "bar".to_string());

        let cases: [(Box<dyn Fn(&str) -> String>, StringSet); 2] = [
            (
                Box::new(|v| format!("mybucket/{}", v)),
                string_set!("mybucket/bar".to_string(), "mybucket/foo".to_string()),
            ),
            (
                Box::new(|v| String::from(v.split_at(1).1)),
                string_set!("ar".to_string(), "oo".to_string()),
            ),
        ];

        for (func, expected_result) in cases {
            let result = ss.apply_fn(func);

            assert_eq!(
                result, expected_result,
                "expected: {}, got: {}",
                expected_result, result
            );
        }
    }

    #[test]
    fn string_set_equals() {
        let cases = [
            (
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "bar".to_string()),
                true,
            ),
            (
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "bar".to_string(), "baz".to_string()),
                false,
            ),
            (
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("bar".to_string()),
                false,
            ),
        ];

        for (set1, set2, expected_result) in cases {
            let result = set1 == set2;

            assert_eq!(
                result, expected_result,
                "expected: {}, got: {}",
                expected_result, result
            );
        }
    }

    #[test]
    fn string_set_intersection() {
        let cases = [
            (
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "bar".to_string()),
            ),
            (
                string_set!("foo".to_string(), "bar".to_string(), "baz".to_string()),
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "bar".to_string()),
            ),
            (
                string_set!("foo".to_string(), "baz".to_string()),
                string_set!("baz".to_string(), "bar".to_string()),
                string_set!("baz".to_string()),
            ),
            (
                string_set!("foo".to_string(), "baz".to_string()),
                string_set!("poo".to_string(), "bar".to_string()),
                string_set!(),
            ),
        ];

        for (set1, set2, expected_result) in cases {
            let result = set1.intersection(&set2);

            assert_eq!(
                result, expected_result,
                "expected: {}, got: {}",
                expected_result, result
            );
        }
    }

    #[test]
    fn string_set_difference() {
        let cases = [
            (
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "bar".to_string()),
                StringSet::new(),
            ),
            (
                string_set!("foo".to_string(), "bar".to_string(), "baz".to_string()),
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("baz".to_string()),
            ),
            (
                string_set!("foo".to_string(), "baz".to_string()),
                string_set!("baz".to_string(), "bar".to_string()),
                string_set!("foo".to_string()),
            ),
            (
                string_set!("foo".to_string(), "baz".to_string()),
                string_set!("poo".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "baz".to_string()),
            ),
        ];

        for (set1, set2, expected_result) in cases {
            let result = set1.difference(&set2);

            assert_eq!(
                result, expected_result,
                "expected: {}, got: {}",
                expected_result, result
            );
        }
    }

    #[test]
    fn string_set_union() {
        let cases = [
            (
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "bar".to_string()),
            ),
            (
                string_set!("foo".to_string(), "bar".to_string(), "baz".to_string()),
                string_set!("foo".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "bar".to_string(), "baz".to_string()),
            ),
            (
                string_set!("foo".to_string(), "baz".to_string()),
                string_set!("baz".to_string(), "bar".to_string()),
                string_set!("foo".to_string(), "baz".to_string(), "bar".to_string()),
            ),
            (
                string_set!("foo".to_string(), "baz".to_string()),
                string_set!("poo".to_string(), "bar".to_string()),
                string_set!(
                    "foo".to_string(),
                    "baz".to_string(),
                    "poo".to_string(),
                    "bar".to_string()
                ),
            ),
        ];

        for (set1, set2, expected_result) in cases {
            let result = set1.union(&set2);

            assert_eq!(
                result, expected_result,
                "expected: {}, got: {}",
                expected_result, result
            );
        }
    }

    #[test]
    fn string_set_serialize_json() {
        let cases = [
            (
                string_set!("foo".to_string(), "bar".to_string()),
                r#"["foo","bar"]"#,
            ),
            (StringSet::new(), r#"[]"#),
        ];

        for (set, expected_result) in cases {
            let result = assert_ok!(serde_json::to_string(&set));
            let result_de: StringSet = assert_ok!(serde_json::from_str(&result));
            let expected_result_de: StringSet = assert_ok!(serde_json::from_str(expected_result));

            assert_eq!(
                result_de, expected_result_de,
                "expected: {}, got: {}",
                expected_result, result
            );
        }
    }

    #[test]
    fn string_set_deserialize_json() {
        let cases = [
            (
                r#"["bar","foo"]"#,
                string_set!("bar".to_string(), "foo".to_string()),
            ),
            (
                r#"["bar","foo"]"#,
                string_set!("bar".to_string(), "foo".to_string()),
            ),
            (r#"[]"#, StringSet::new()),
            (r#""""#, string_set!("".to_string())),
        ];

        for (data, expected_result) in cases {
            let result: StringSet = assert_ok!(serde_json::from_str(data));

            assert_eq!(
                result, expected_result,
                "expected: {}, got: {}",
                expected_result, result
            );
        }
    }

    #[test]
    fn string_set_to_vec() {
        let cases = [
            (StringSet::new(), vec![]),
            (string_set!("".to_string()), vec!["".to_string()]),
            (string_set!("foo".to_string()), vec!["foo".to_string()]),
            (
                string_set!("foo".to_string(), "bar".to_string()),
                vec!["bar".to_string(), "foo".to_string()],
            ),
        ];

        for (set, expected_result) in cases {
            let result = set.to_vec();

            assert_eq!(
                result, expected_result,
                "expected: {:?}, got: {:?}",
                expected_result, result
            );
        }
    }
}
