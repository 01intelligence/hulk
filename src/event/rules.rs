use std::collections::{HashMap, HashSet};

use super::*;

pub fn new_pattern(prefix: &str, suffix: &str) -> String {
    let mut pattern = String::new();
    if !prefix.is_empty() {
        pattern += prefix;
        if !prefix.ends_with('*') {
            pattern += "*";
        }
    }
    if !suffix.is_empty() {
        if !suffix.starts_with('*') {
            pattern += "*";
        }
        pattern += suffix;
    }
    pattern.replace("**", "*")
}

// Event rules.
#[derive(Default, Clone)]
pub struct Rules(HashMap<String, HashSet<TargetId>>);

impl Rules {
    pub fn add(&mut self, pattern: String, target_id: TargetId) {
        let _ = self
            .0
            .entry(pattern)
            .or_insert_with(|| Default::default())
            .insert(target_id);
    }

    pub fn match_simple(&self, object_name: &str) -> bool {
        for pattern in self.0.keys() {
            if crate::wildcard::match_wildcard_simple(pattern, object_name) {
                return true;
            }
        }
        false
    }

    pub fn match_simple_targets(&self, object_name: &str) -> HashSet<TargetId> {
        let mut matched_targets = HashSet::new();
        for (pattern, targets) in &self.0 {
            if crate::wildcard::match_wildcard_simple(pattern, object_name) {
                for target in targets {
                    matched_targets.get_or_insert_owned(target);
                }
            }
        }
        matched_targets
    }

    pub fn union(&mut self, other: Rules) {
        for (pattern, targets) in other.0 {
            let mut v = self.0.entry(pattern).or_insert_with(|| Default::default());
            for t in targets {
                if !v.contains(&t) {
                    v.insert(t);
                }
            }
        }
    }

    pub fn difference(&mut self, other: Rules) {
        for (pattern, targets) in other.0 {
            if let Some(v) = self.0.get_mut(&pattern) {
                for t in targets {
                    if v.contains(&t) {
                        v.remove(&t);
                    }
                }
                if v.is_empty() {
                    self.0.remove(&pattern);
                }
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct RulesMap(HashMap<Name, Rules>);

impl RulesMap {
    pub fn new(event_names: &[&Name], mut pattern: String, target: TargetId) -> RulesMap {
        // If pattern is empty, add '*' wildcard to match all.
        if pattern.is_empty() {
            pattern = "*".to_owned();
        }

        let mut rules = Rules::default();
        rules.add(pattern, target);

        let mut rules_map = RulesMap::default();
        for name in event_names {
            for name in name.expand() {
                let r = rules_map
                    .0
                    .entry(name)
                    .or_insert_with(|| Default::default());
                r.union(rules.clone());
            }
        }
        rules_map
    }

    pub fn add(&mut self, other: RulesMap) {
        for (name, rules) in other.0 {
            let r = self.0.entry(name).or_insert_with(|| Default::default());
            r.union(rules);
        }
    }

    pub fn remove(&mut self, other: RulesMap) {
        for (name, rules) in other.0 {
            if let Some(v) = self.0.get_mut(&name) {
                v.difference(rules);
                if v.0.is_empty() {
                    self.0.remove(&name);
                }
            }
        }
    }

    pub fn match_simple(&self, event_name: &Name, object_name: &str) -> bool {
        self.0
            .get(event_name)
            .map(|r| r.match_simple(object_name))
            .unwrap_or_else(|| false)
    }

    pub fn match_simple_targets(&self, event_name: &Name, object_name: &str) -> HashSet<TargetId> {
        self.0
            .get(event_name)
            .map(|r| r.match_simple_targets(object_name))
            .unwrap_or_else(|| Default::default())
    }
}
