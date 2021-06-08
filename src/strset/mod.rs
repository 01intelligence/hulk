use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct StringSet(HashSet<String>);

impl StringSet {
    pub fn new() -> StringSet {
        StringSet(HashSet::new())
    }

    pub fn from_slice(ss: &[&str]) -> StringSet {
        StringSet(ss.iter().map(|&s| s.into()).collect())
    }

    pub fn as_slice(&self) -> Vec<&str> {
        let mut ss: Vec<&str> = self.0.iter().map(|s| s as &str).collect();
        ss.sort_unstable();
        ss
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

    pub fn match_fn<F>(&self, match_fn: F, match_str: &str) -> StringSet
    where
        F: Fn(&str, &str) -> bool,
    {
        StringSet(
            self.0
                .iter()
                .filter(|&s| match_fn(s, match_str))
                .cloned()
                .collect(),
        )
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

impl ToString for StringSet {
    fn to_string(&self) -> String {
        format!("{:?}", self.as_slice())
    }
}
