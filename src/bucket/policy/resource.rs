use std::collections::HashMap;

use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::bucket::policy::Valid;

// Resource ARN prefix as per AWS S3 specification.
pub const RESOURCE_ARN_PREFIX: &str = "arn:aws:s3:::";

// Resource in policy statement.
#[derive(Serialize, Deserialize)]
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
    pub fn is_match(&self, resource: &str, condition_values: HashMap<String, Vec<String>>) -> bool {
        let mut pattern = self.pattern.clone();
        todo!()
    }

    // Validates resource is for given bucket or not.
    fn validate_bucket(&self, bucket_name: &str) -> anyhow::Result<()> {
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

impl super::Valid for Resource {
    fn is_valid(&self) -> bool {
        !self.bucket_name.is_empty() && !self.pattern.is_empty()
    }
}

impl ToString for Resource {
    fn to_string(&self) -> String {
        RESOURCE_ARN_PREFIX.to_owned() + &self.pattern
    }
}
