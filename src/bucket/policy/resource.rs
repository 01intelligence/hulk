use std::collections::{HashMap, HashSet};
use std::fmt;

use anyhow::bail;
use serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};

use super::Valid;

// Resource ARN prefix as per AWS S3 specification.
pub const RESOURCE_ARN_PREFIX: &str = "arn:aws:s3:::";

// Resource in policy statement.
#[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Resource {
    pub bucket_name: String,
    pub pattern: String,
}

impl Resource {
    pub fn new(bucket_name: String, key_name: String) -> Resource {
        let mut pattern = bucket_name.clone();
        if !key_name.is_empty() {
            if !key_name.starts_with('/') {
                pattern += "/";
            }
            pattern += &key_name;
        }
        Resource {
            bucket_name,
            pattern,
        }
    }

    // Matches object name with resource pattern.
    pub fn is_match(
        &self,
        resource: &str,
        condition_values: &HashMap<String, Vec<String>>,
    ) -> bool {
        let mut pattern = self.pattern.clone();
        for key in super::condition::COMMON_KEYS.iter() {
            // Empty values are not supported for policy variables.
            if let Some(rvalues) = condition_values.get(key.name()) {
                if !rvalues.is_empty() && rvalues[0] != "" {
                    pattern = pattern.replace(&key.var_name(), &rvalues[0])
                }
            }
        }
        crate::wildcard::match_wildcard(&pattern, resource)
    }

    // Validates resource is for given bucket or not.
    fn validate(&self, bucket_name: &str) -> anyhow::Result<()> {
        if !self.is_valid() {
            bail!("invalid resource");
        }
        if !crate::wildcard::match_wildcard(&self.bucket_name, bucket_name) {
            bail!("bucket name does not match");
        }
        Ok(())
    }

    fn is_bucket_pattern(&self) -> bool {
        !self.pattern.contains('/')
    }

    fn is_object_pattern(&self) -> bool {
        self.pattern.contains('/') || self.bucket_name.contains('*')
    }
}

impl Valid for Resource {
    fn is_valid(&self) -> bool {
        !self.bucket_name.is_empty() && !self.pattern.is_empty()
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", RESOURCE_ARN_PREFIX, self.pattern)
    }
}

impl Serialize for Resource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if !self.is_valid() {
            return Err(S::Error::custom(format!("invalid resource '{}'", self)));
        }
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Resource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ResourceVisitor;
        impl<'de> Visitor<'de> for ResourceVisitor {
            type Value = Resource;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a resource string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                parse_resource(v).map_err(|e| E::custom(e))
            }
        }

        deserializer.deserialize_str(ResourceVisitor)
    }
}

fn parse_resource(s: &str) -> anyhow::Result<Resource> {
    if !s.starts_with(RESOURCE_ARN_PREFIX) {
        bail!("invalid resource '{}'", s);
    }
    let pattern = s.strip_prefix(RESOURCE_ARN_PREFIX).unwrap();
    let tokens: Vec<&str> = pattern.splitn(2, "/").collect();
    let bucket_name = tokens[0];
    if bucket_name.is_empty() {
        bail!("invalid resource '{}'", s);
    }
    Ok(Resource {
        bucket_name: bucket_name.to_owned(),
        pattern: pattern.to_owned(),
    })
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

    pub fn validate(&self, bucket_name: &str) -> anyhow::Result<()> {
        for r in &self.0 {
            r.validate(bucket_name)?;
        }
        Ok(())
    }
}

impl fmt::Display for ResourceSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut resources = self.0.iter().collect::<Vec<&Resource>>();
        resources.sort_unstable();
        write!(f, "{:?}", resources)
    }
}

impl Serialize for ResourceSet {
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

impl<'de> Deserialize<'de> for ResourceSet {
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
                let r = parse_resource(v).map_err(|e| E::custom(e))?;
                Ok(ResourceSet::new(vec![r]))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_set_serialize_json() {
        let cases = vec![
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_owned(),
                    "/myobject*".to_owned(),
                )]),
                r#"["arn:aws:s3:::mybucket/myobject*"]"#,
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_owned(),
                    "/photos/myobject*".to_owned(),
                )]),
                r#"["arn:aws:s3:::mybucket/photos/myobject*"]"#,
                false,
            ),
            (ResourceSet::new(vec![]), "", true),
        ];
        for (resource_set, expected_result, expect_err) in cases {
            match serde_json::to_string(&resource_set) {
                Ok(result) => {
                    assert!(!expect_err);
                    assert_eq!(result, expected_result);
                }
                Err(_) => {
                    assert!(expect_err);
                }
            }
        }
    }

    #[test]
    fn test_resource_set_deserialize_json() {
        let cases = vec![
            (
                r#""arn:aws:s3:::mybucket/myobject*""#,
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_owned(),
                    "/myobject*".to_owned(),
                )]),
                false,
            ),
            (
                r#""arn:aws:s3:::mybucket/photos/myobject*""#,
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_owned(),
                    "/photos/myobject*".to_owned(),
                )]),
                false,
            ),
            (
                r#""arn:aws:s3:::mybucket""#,
                ResourceSet::new(vec![Resource::new("mybucket".to_owned(), "".to_owned())]),
                false,
            ),
            (
                r#"["arn:aws:s3:::mybucket"]"#,
                ResourceSet::new(vec![Resource::new("mybucket".to_owned(), "".to_owned())]),
                false,
            ),
            (r#""mybucket/myobject*""#, ResourceSet::new(vec![]), true),
        ];
        for (data, expected_result, expect_err) in cases {
            match serde_json::from_str::<ResourceSet>(data) {
                Ok(result) => {
                    assert!(!expect_err);
                    assert_eq!(result, expected_result);
                }
                Err(_) => {
                    assert!(expect_err);
                }
            }
        }
    }

    // TODO
}
