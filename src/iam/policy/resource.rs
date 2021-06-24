use std::collections::{HashMap, HashSet};
use std::fmt;

use anyhow::bail;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{self, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

use crate::bucket::policy as bpolicy;
use crate::bucket::policy::Valid;

// Resource in policy statement.
#[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Debug, Serialize, Deserialize)]
pub struct Resource(bpolicy::Resource);

impl Resource {
    pub fn new(bucket_name: String, key_name: String) -> Resource {
        Resource(bpolicy::Resource::new(bucket_name, key_name))
    }

    // Matches object name with resource pattern only.
    pub fn is_match_resource(&self, resource: &str) -> bool {
        self.is_match(resource, &HashMap::default())
    }

    // Matches object name with resource pattern.
    pub fn is_match(
        &self,
        resource: &str,
        condition_values: &HashMap<String, Vec<String>>,
    ) -> bool {
        let mut pattern = self.0.pattern.clone();
        for key in bpolicy::condition::COMMON_KEYS.iter() {
            // Empty values are not supported for policy variables.
            if let Some(rvalues) = condition_values.get(key.name()) {
                if !rvalues.is_empty() && rvalues[0] != "" {
                    pattern = pattern.replace(&key.var_name(), &rvalues[0])
                }
            }
        }
        let cp = path_clean::clean(resource);
        if cp != "." && cp == pattern {
            return true;
        }
        crate::wildcard::match_wildcard(&pattern, resource)
    }

    fn validate(&self) -> anyhow::Result<()> {
        if !self.is_valid() {
            bail!("invalid resource");
        }
        Ok(())
    }

    fn is_bucket_pattern(&self) -> bool {
        !self.0.pattern.contains('/') || self.0.pattern == "*"
    }

    fn is_object_pattern(&self) -> bool {
        self.0.pattern.contains('/') || self.0.bucket_name.contains('*') || self.0.pattern == "*/*"
    }
}

impl bpolicy::Valid for Resource {
    fn is_valid(&self) -> bool {
        !self.0.pattern.is_empty()
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Set of resources in policy statement.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct ResourceSet(HashSet<Resource>);

impl ResourceSet {
    pub fn new(resources: Vec<Resource>) -> ResourceSet {
        ResourceSet(resources.into_iter().collect())
    }

    pub fn add(&mut self, resource: Resource) {
        self.0.insert(resource);
    }

    pub fn intersection(&self, set: &ResourceSet) -> ResourceSet {
        ResourceSet(self.0.intersection(&set.0).cloned().collect())
    }

    // Matches object name with resource patterns only.
    pub fn is_match_resource(&self, resource: &str) -> bool {
        self.0.iter().any(|r| r.is_match_resource(resource))
    }

    // Matches object name with anyone of resource pattern in resource set.
    pub fn is_match(
        &self,
        resource: &str,
        condition_values: &HashMap<String, Vec<String>>,
    ) -> bool {
        self.0
            .iter()
            .any(|r| r.is_match(resource, condition_values))
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        for r in &self.0 {
            r.validate()?;
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    // Checks if at least one bucket resource exists in the set.
    pub(super) fn bucket_resource_exists(&self) -> bool {
        self.0.iter().any(|r| r.is_bucket_pattern())
    }

    // Checks if at least one object resource exists in the set.
    pub(super) fn object_resource_exists(&self) -> bool {
        self.0.iter().any(|r| r.is_object_pattern())
    }
}

impl fmt::Display for ResourceSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut resources = self.0.iter().collect::<Vec<&Resource>>();
        resources.sort_unstable();
        write!(f, "{:?}", resources)
    }
}

impl ser::Serialize for ResourceSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if self.0.is_empty() {
            return Err(S::Error::custom("empty resource set"));
        }
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for r in &self.0 {
            seq.serialize_element(r)?;
        }
        seq.end()
    }
}

impl<'de> de::Deserialize<'de> for ResourceSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ResourceSetVisitor;
        impl<'de> Visitor<'de> for ResourceSetVisitor {
            type Value = ResourceSet;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a resource array or a resource")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let r = bpolicy::parse_resource(v).map_err(|e| E::custom(e))?;
                Ok(ResourceSet::new(vec![Resource(r)]))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                use serde::de::Error;
                let mut set = ResourceSet::new(vec![]);
                while let Some(v) = seq.next_element()? {
                    if set.0.contains(&v) {
                        return Err(A::Error::custom(format!("duplicate value found '{}'", v)));
                    }
                    set.add(v);
                }
                Ok(set)
            }
        }

        deserializer.deserialize_any(ResourceSetVisitor)
    }
}
