use std::collections::{HashMap, HashSet};
use std::fmt;

use anyhow::bail;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{self, Serialize, SerializeSeq, Serializer};
use serde::Deserialize;

use crate::bucket::policy as bpolicy;
use crate::bucket::policy::Valid;

// Resource in policy statement.
#[derive(Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Debug, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::assert::*;

    #[test]
    fn test_resource_is_bucket_pattern() {
        let cases = [
            (Resource::new("*".to_string(), "".to_string()), true),
            (Resource::new("mybucket".to_string(), "".to_string()), true),
            (Resource::new("mybucket*".to_string(), "".to_string()), true),
            (
                Resource::new("mybucket?0".to_string(), "".to_string()),
                true,
            ),
            (Resource::new("".to_string(), "*".to_string()), false),
            (Resource::new("*".to_string(), "*".to_string()), false),
            (
                Resource::new("mybucket".to_string(), "*".to_string()),
                false,
            ),
            (
                Resource::new("mybucket*".to_string(), "/myobject".to_string()),
                false,
            ),
            (
                Resource::new("mybucket?0".to_string(), "/2021/photos/*".to_string()),
                false,
            ),
        ];

        for (resource, expected_result) in cases {
            let result = resource.is_bucket_pattern();

            assert_eq!(
                result, expected_result,
                "resource: {}, expected: {}, got: {}",
                resource, expected_result, result
            );
        }
    }

    #[test]
    fn test_resource_is_object_pattern() {
        let cases = [
            (Resource::new("*".to_string(), "".to_string()), true),
            (Resource::new("mybucket*".to_string(), "".to_string()), true),
            (Resource::new("".to_string(), "*".to_string()), true),
            (Resource::new("*".to_string(), "*".to_string()), true),
            (Resource::new("mybucket".to_string(), "*".to_string()), true),
            (
                Resource::new("mybucket*".to_string(), "/myobject".to_string()),
                true,
            ),
            (
                Resource::new("mybucket?0".to_string(), "/2021/photos/*".to_string()),
                true,
            ),
            (Resource::new("mybucket".to_string(), "".to_string()), false),
            (
                Resource::new("mybucket?0".to_string(), "".to_string()),
                false,
            ),
        ];

        for (resource, expected_result) in cases {
            let result = resource.is_object_pattern();

            assert_eq!(
                result, expected_result,
                "resource: {}, expected: {}, got: {}",
                resource, expected_result, result
            );
        }
    }

    #[test]
    fn test_resource_match() {
        let cases = [
            (
                Resource::new("*".to_string(), "".to_string()),
                "mybucket",
                true,
            ),
            (
                Resource::new("*".to_string(), "".to_string()),
                "mybucket/myobject",
                true,
            ),
            (
                Resource::new("mybucket*".to_string(), "".to_string()),
                "mybucket",
                true,
            ),
            (
                Resource::new("mybucket*".to_string(), "".to_string()),
                "mybucket/myobject",
                true,
            ),
            (
                Resource::new("".to_string(), "*".to_string()),
                "/myobject",
                true,
            ),
            (
                Resource::new("*".to_string(), "*".to_string()),
                "mybucket/myobject",
                true,
            ),
            (
                Resource::new("mybucket".to_string(), "*".to_string()),
                "mybucket/myobject",
                true,
            ),
            (
                Resource::new("mybucket*".to_string(), "/myobject".to_string()),
                "mybucket/myobject",
                true,
            ),
            (
                Resource::new("mybucket*".to_string(), "/myobject".to_string()),
                "mybucket100/myobject",
                true,
            ),
            (
                Resource::new("mybucket?0".to_string(), "/2021/photos/*".to_string()),
                "mybucket20/2021/photos/1.jpg",
                true,
            ),
            (
                Resource::new("mybucket".to_string(), "".to_string()),
                "mybucket",
                true,
            ),
            (
                Resource::new("mybucket?0".to_string(), "".to_string()),
                "mybucket30",
                true,
            ),
            (
                Resource::new("".to_string(), "*".to_string()),
                "mybucket/myobject",
                false,
            ),
            (
                Resource::new("*".to_string(), "*".to_string()),
                "mybucket",
                false,
            ),
            (
                Resource::new("mybucket".to_string(), "*".to_string()),
                "mybucket10/myobject",
                false,
            ),
            (
                Resource::new("mybucket?0".to_string(), "/2021/photos/*".to_string()),
                "mybucket0/2021/photos/1.jpg",
                false,
            ),
            (
                Resource::new("mybucket".to_string(), "".to_string()),
                "mybucket/myobject",
                false,
            ),
        ];

        for (resource, object_name, expected_result) in cases {
            let result = resource.is_match(object_name, &HashMap::new());

            assert_eq!(
                result, expected_result,
                "resource: {}, expected: {}, got: {}",
                resource, expected_result, result
            );
        }
    }

    #[test]
    fn test_resource_serialize_json() {
        let cases = [
            (
                Resource::new("*".to_string(), "".to_string()),
                r#""arn:aws:s3:::*""#,
            ),
            (
                Resource::new("mybucket*".to_string(), "".to_string()),
                r#""arn:aws:s3:::mybucket*""#,
            ),
            (
                Resource::new("mybucket".to_string(), "".to_string()),
                r#""arn:aws:s3:::mybucket""#,
            ),
            (
                Resource::new("*".to_string(), "*".to_string()),
                r#""arn:aws:s3:::*/*""#,
            ),
            (
                Resource::new("".to_string(), "*".to_string()),
                r#""arn:aws:s3:::/*""#,
            ),
            (
                Resource::new("mybucket".to_string(), "*".to_string()),
                r#""arn:aws:s3:::mybucket/*""#,
            ),
            (
                Resource::new("mybucket*".to_string(), "myobject".to_string()),
                r#""arn:aws:s3:::mybucket*/myobject""#,
            ),
            (
                Resource::new("mybucket?0".to_string(), "/2021/photos/*".to_string()),
                r#""arn:aws:s3:::mybucket?0/2021/photos/*""#,
            ),
        ];

        for (resource, expected_result) in cases {
            let result = assert_ok!(serde_json::to_string(&resource));

            assert_eq!(
                result, expected_result,
                "resource: {}, expected: {}, got: {}",
                resource, expected_result, result
            );
        }
    }

    #[test]
    fn test_resource_deserialize_json() {
        let cases = [
            (
                r#""arn:aws:s3:::*""#,
                Some(Resource::new("*".to_string(), "".to_string())),
                false,
            ),
            (
                r#""arn:aws:s3:::mybucket*""#,
                Some(Resource::new("mybucket*".to_string(), "".to_string())),
                false,
            ),
            (
                r#""arn:aws:s3:::mybucket""#,
                Some(Resource::new("mybucket".to_string(), "".to_string())),
                false,
            ),
            (
                r#""arn:aws:s3:::*/*""#,
                Some(Resource::new("*".to_string(), "*".to_string())),
                false,
            ),
            (
                r#""arn:aws:s3:::mybucket/*""#,
                Some(Resource::new("mybucket".to_string(), "*".to_string())),
                false,
            ),
            (
                r#""arn:aws:s3:::mybucket*/myobject""#,
                Some(Resource::new(
                    "mybucket*".to_string(),
                    "myobject".to_string(),
                )),
                false,
            ),
            (
                r#""arn:aws:s3:::mybucket?0/2021/photos/*""#,
                Some(Resource::new(
                    "mybucket?0".to_string(),
                    "/2021/photos/*".to_string(),
                )),
                false,
            ),
            (r#""mybucket/myobject*""#, None, true),
            (r#""arn:aws:s3:::/*""#, None, true),
        ];

        for (data, expected_result, expect_err) in cases {
            let result = serde_json::from_str::<Resource>(data);

            match result {
                Ok(result) => {
                    if let Some(expected_result) = expected_result {
                        assert_eq!(
                            result, expected_result,
                            "data: {}, expected: {}, got: {}",
                            data, expected_result, result
                        );
                    }
                }
                Err(_) => assert!(expect_err, "expect an error"),
            }
        }
    }

    #[test]
    fn test_resource_is_valid() {
        let cases = [
            (Resource::new("*".to_string(), "".to_string()), true),
            (Resource::new("*".to_string(), "*".to_string()), true),
            (Resource::new("mybucket*".to_string(), "".to_string()), true),
            (Resource::new("mybucket".to_string(), "*".to_string()), true),
            (
                Resource::new("mybucket*".to_string(), "/myobject".to_string()),
                true,
            ),
            (
                Resource::new("mybucket?0".to_string(), "/2021/photos/*".to_string()),
                true,
            ),
            (Resource::new("mybucket".to_string(), "".to_string()), true),
            (
                Resource::new("mybucket?0".to_string(), "".to_string()),
                true,
            ),
            (Resource::new("".to_string(), "*".to_string()), true),
            (Resource::new("".to_string(), "".to_string()), false),
        ];

        for (resource, expected_result) in cases {
            let result = resource.is_valid();

            assert_eq!(
                result, expected_result,
                "resource: {}, expected: {}, got: {}",
                resource, expected_result, result
            );
        }
    }

    #[test]
    fn test_resource_validate() {
        let cases = [
            (
                Resource::new("mybucket".to_string(), "/myobject*".to_string()),
                false,
            ),
            (
                Resource::new("".to_string(), "/myobject*".to_string()),
                false,
            ),
            (Resource::new("".to_string(), "".to_string()), true),
        ];

        for (resource, expect_err) in cases {
            if expect_err {
                assert_err!(resource.validate());
            } else {
                assert_ok!(resource.validate())
            }
        }
    }

    #[test]
    fn test_resource_set_bucket_resource_exists() {
        let cases = [
            (
                ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket".to_string(), "".to_string())]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket*".to_string(), "".to_string())]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket?0".to_string(),
                    "".to_string(),
                )]),
                true,
            ),
            (
                ResourceSet::new(vec![
                    Resource::new("mybucket".to_string(), "/2021/photos/*".to_string()),
                    Resource::new("mybucket".to_string(), "".to_string()),
                ]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("".to_string(), "*".to_string())]),
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new("*".to_string(), "*".to_string())]),
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket".to_string(), "*".to_string())]),
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket*".to_string(),
                    "/myobcket".to_string(),
                )]),
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket?0".to_string(),
                    "/2021/photos/*".to_string(),
                )]),
                false,
            ),
        ];

        for (set, expected_result) in cases {
            let result = set.bucket_resource_exists();

            assert_eq!(
                result, expected_result,
                "set: {}; expected: {}, got: {}",
                set, expected_result, result
            );
        }
    }

    #[test]
    fn test_resource_set_object_resource_exists() {
        let cases = [
            (
                ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket*".to_string(), "".to_string())]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("".to_string(), "*".to_string())]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("*".to_string(), "*".to_string())]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket".to_string(), "*".to_string())]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket*".to_string(),
                    "/myobject".to_string(),
                )]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket?0".to_string(),
                    "/2021/photos/*".to_string(),
                )]),
                true,
            ),
            (
                ResourceSet::new(vec![
                    Resource::new("mybucket".to_string(), "".to_string()),
                    Resource::new("mybucket".to_string(), "/2021/photos/*".to_string()),
                ]),
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket".to_string(), "".to_string())]),
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket?0".to_string(),
                    "".to_string(),
                )]),
                false,
            ),
        ];

        for (set, expected_result) in cases {
            let result = set.object_resource_exists();

            assert_eq!(
                result, expected_result,
                "set: {}; expected: {}, got: {}",
                set, expected_result, result
            );
        }
    }

    #[test]
    fn test_resource_set_add() {
        let cases = [
            (
                ResourceSet::new(vec![]),
                Resource::new("mybucket".to_string(), "/myobject*".to_string()),
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                Resource::new("mybucket".to_string(), "/yourobject*".to_string()),
                ResourceSet::new(vec![
                    Resource::new("mybucket".to_string(), "/myobject*".to_string()),
                    Resource::new("mybucket".to_string(), "/yourobject*".to_string()),
                ]),
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                Resource::new("mybucket".to_string(), "/myobject*".to_string()),
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
            ),
        ];

        for (mut set, resource, expected_result) in cases {
            set.add(resource);

            assert_eq!(set, expected_result)
        }
    }

    #[test]
    fn test_resource_set_intersection() {
        let cases = [
            (
                ResourceSet::new(vec![]),
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                ResourceSet::new(vec![]),
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                ResourceSet::new(vec![]),
                ResourceSet::new(vec![]),
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                ResourceSet::new(vec![
                    Resource::new("mybucket".to_string(), "/myobject*".to_string()),
                    Resource::new("mybucket".to_string(), "/yourobject*".to_string()),
                ]),
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
            ),
        ];

        for (set, set_to_intersect, expected_result) in cases {
            let result = set.intersection(&set_to_intersect);

            assert_eq!(
                result, expected_result,
                "set: {}, expected: {}, got: {}",
                set, expected_result, result
            );
        }
    }

    #[test]
    fn test_resource_set_match() {
        let cases = [
            (
                ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
                "mybucket",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("*".to_string(), "".to_string())]),
                "mybucket/myobject",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket*".to_string(), "".to_string())]),
                "mybucket",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket*".to_string(), "".to_string())]),
                "mybucket/myobject",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("".to_string(), "*".to_string())]),
                "/myobject",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("*".to_string(), "*".to_string())]),
                "mybucket/myobject",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket".to_string(), "*".to_string())]),
                "mybucket/myobject",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket*".to_string(),
                    "/myobject".to_string(),
                )]),
                "mybucket/myobject",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket*".to_string(),
                    "/myobject".to_string(),
                )]),
                "mybucket10/myobject",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket?0".to_string(),
                    "/2021/photos/*".to_string(),
                )]),
                "mybucket20/2021/photos/1.jpg",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket".to_string(), "".to_string())]),
                "mybucket",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket?0".to_string(),
                    "".to_string(),
                )]),
                "mybucket30",
                true,
            ),
            (
                ResourceSet::new(vec![
                    Resource::new("mybucket?0".to_string(), "/2021/photos/*".to_string()),
                    Resource::new("mybucket".to_string(), "/2021/photos/*".to_string()),
                ]),
                "mybucket/2021/photos/1.jpg",
                true,
            ),
            (
                ResourceSet::new(vec![Resource::new("".to_string(), "*".to_string())]),
                "mybucket/object",
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new("*".to_string(), "*".to_string())]),
                "mybucket",
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket".to_string(), "*".to_string())]),
                "mybucket10/myobject",
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new("mybucket".to_string(), "".to_string())]),
                "mybucket/myobject",
                false,
            ),
            (ResourceSet::new(vec![]), "mybucket/myobject", false),
        ];

        for (set, resource, expected_result) in cases {
            let result = set.is_match_resource(resource);

            assert_eq!(
                result, expected_result,
                "set: {}, expected: {}, got: {}",
                set, expected_result, result
            );
        }
    }

    #[test]
    fn test_resource_set_serialize_json() {
        let cases = [
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                r#"["arn:aws:s3:::mybucket/myobject*"]"#,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/2021/photos/*".to_string(),
                )]),
                r#"["arn:aws:s3:::mybucket/2021/photos/*"]"#,
            ),
        ];

        for (set, expected_result) in cases {
            let result = assert_ok!(serde_json::to_string(&set));

            assert_eq!(
                result, expected_result,
                "set: {}, expected: {}, got: {}",
                set, expected_result, result
            );
        }
    }

    #[test]
    fn test_resource_set_deserialize_json() {
        let cases = [
            (
                r#""arn:aws:s3:::mybucket/myobject*""#,
                Some(ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )])),
                false,
            ),
            (
                r#""arn:aws:s3:::mybucket/2021/photos/*""#,
                Some(ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/2021/photos/*".to_string(),
                )])),
                false,
            ),
            (
                r#""arn:aws:s3:::mybucket""#,
                Some(ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "".to_string(),
                )])),
                false,
            ),
            (r#""mybucket/myobject*""#, None, true),
        ];

        for (data, expected_result, expect_err) in cases {
            let result = serde_json::from_str::<ResourceSet>(data);

            match result {
                Ok(result) => {
                    let result = if expect_err { None } else { Some(result) };
                    assert_eq!(result, expected_result);
                }
                Err(_) => assert!(expect_err, "expect an error"),
            }
        }
    }

    #[test]
    fn test_resource_set_validate() {
        let cases = [
            (
                ResourceSet::new(vec![Resource::new(
                    "mybucket".to_string(),
                    "/myobject*".to_string(),
                )]),
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new(
                    "".to_string(),
                    "/myobject*".to_string(),
                )]),
                false,
            ),
            (
                ResourceSet::new(vec![Resource::new("".to_string(), "".to_string())]),
                true,
            ),
        ];

        for (set, expect_err) in cases {
            if !expect_err {
                assert_ok!(set.validate());
            } else {
                assert_err!(set.validate());
            }
        }
    }
}
