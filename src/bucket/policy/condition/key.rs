use std::collections::{HashMap, HashSet};
use std::fmt;

use lazy_static::lazy_static;
use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::bucket::policy::{ToVec, Valid};

// Conditional key which is used to fetch values for any condition.
// Refer https://docs.aws.amazon.com/IAM/latest/UserGuide/list_s3.html
// for more information about available condition keys.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Key<'a>(pub &'a str);

// S3X_AMZ_COPY_SOURCE - key representing x-amz-copy-source HTTP header applicable to PutObject API only.
pub const S3X_AMZ_COPY_SOURCE: Key = Key("s3:x-amz-copy-source");

// S3X_AMZ_SERVER_SIDE_ENCRYPTION - key representing x-amz-server-side-encryption HTTP header applicable
// to PutObject API only.
pub const S3X_AMZ_SERVER_SIDE_ENCRYPTION: Key = Key("s3:x-amz-server-side-encryption");

// S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM - key representing
// x-amz-server-side-encryption-customer-algorithm HTTP header applicable to PutObject API only.
pub const S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM: Key =
    Key("s3:x-amz-server-side-encryption-customer-algorithm");

// S3X_AMZ_METADATA_DIRECTIVE - key representing x-amz-metadata-directive HTTP header applicable to
// PutObject API only.
pub const S3X_AMZ_METADATA_DIRECTIVE: Key = Key("s3:x-amz-metadata-directive");

// S3X_AMZ_CONTENT_SHA256 - set a static content-sha256 for all calls for a given action.
pub const S3X_AMZ_CONTENT_SHA256: Key = Key("s3:x-amz-content-sha256");

// S3X_AMZ_STORAGE_CLASS - key representing x-amz-storage-class HTTP header applicable to PutObject API
// only.
pub const S3X_AMZ_STORAGE_CLASS: Key = Key("s3:x-amz-storage-class");

// S3_LOCATION_CONSTRAINT - key representing LocationConstraint XML tag of CreateBucket API only.
pub const S3_LOCATION_CONSTRAINT: Key = Key("s3:LocationConstraint");

// S3_PREFIX - key representing prefix query parameter of ListBucket API only.
pub const S3_PREFIX: Key = Key("s3:prefix");

// S3_DELIMITER - key representing delimiter query parameter of ListBucket API only.
pub const S3_DELIMITER: Key = Key("s3:delimiter");

// S3_VERSION_ID - Enables you to limit the permission for the
// s3:PutObjectVersionTagging action to a specific object version.
pub const S3_VERSION_ID: Key = Key("s3:versionid");

// S3_MAX_KEYS - key representing max-keys query parameter of ListBucket API only.
pub const S3_MAX_KEYS: Key = Key("s3:max-keys");

// S3_OBJECT_LOCK_REMAINING_RETENTION_DAYS - key representing object-lock-remaining-retention-days
// Enables enforcement of an object relative to the remaining retention days, you can set
// minimum and maximum allowable retention periods for a bucket using a bucket policy.
// This key are specific for s3:PutObjectRetention API.
pub const S3_OBJECT_LOCK_REMAINING_RETENTION_DAYS: Key =
    Key("s3:object-lock-remaining-retention-days");

// S3_OBJECT_LOCK_MODE - key representing object-lock-mode
// Enables enforcement of the specified object retention mode
pub const S3_OBJECT_LOCK_MODE: Key = Key("s3:object-lock-mode");

// S3_OBJECT_LOCK_RETAIN_UNTIL_DATE - key representing object-lock-retain-util-date
// Enables enforcement of a specific retain-until-date
pub const S3_OBJECT_LOCK_RETAIN_UNTIL_DATE: Key = Key("s3:object-lock-retain-until-date");

// S3_OBJECT_LOCK_LEGAL_HOLD - key representing object-local-legal-hold
// Enables enforcement of the specified object legal hold status
pub const S3_OBJECT_LOCK_LEGAL_HOLD: Key = Key("s3:object-lock-legal-hold");

// AWS_REFERER - key representing Referer header of any API.
pub const AWS_REFERER: Key = Key("aws:Referer");

// AWS_SOURCE_IP - key representing client's IP address (not intermittent proxies) of any API.
pub const AWS_SOURCE_IP: Key = Key("aws:SourceIp");

// AWS_USER_AGENT - key representing UserAgent header for any API.
pub const AWS_USER_AGENT: Key = Key("aws:UserAgent");

// AWS_SECURE_TRANSPORT - key representing if the clients request is authenticated or not.
pub const AWS_SECURE_TRANSPORT: Key = Key("aws:SecureTransport");

// AWS_CURRENT_TIME - key representing the current time.
pub const AWS_CURRENT_TIME: Key = Key("aws:CurrentTime");

// AWS_EPOCH_TIME - key representing the current epoch time.
pub const AWS_EPOCH_TIME: Key = Key("aws:EpochTime");

// AWS_PRINCIPAL_TYPE - user principal type currently supported values are "User" and "Anonymous".
pub const AWS_PRINCIPAL_TYPE: Key = Key("aws:principaltype");

// AWS_USER_ID - user unique ID, in hulk this value is same as your user Access Key.
pub const AWS_USER_ID: Key = Key("aws:userid");

// AWS_USERNAME - user friendly name, in hulk this value is same as your user Access Key.
pub const AWS_USERNAME: Key = Key("aws:username");

// S3_SIGNATURE_VERSION - identifies the version of AWS Signature that you want to support for authenticated requests.
pub const S3_SIGNATURE_VERSION: Key = Key("s3:signatureversion");

// S3_AUTH_TYPE - optionally use this condition key to restrict incoming requests to use a specific authentication method.
pub const S3_AUTH_TYPE: Key = Key("s3:authType");

lazy_static! {
    // List of all all supported keys.
    pub static ref ALL_SUPPORTED_KEYS: Vec<Key<'static>> = {
        let mut keys = vec![
            S3X_AMZ_COPY_SOURCE,
            S3X_AMZ_SERVER_SIDE_ENCRYPTION,
            S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM,
            S3X_AMZ_METADATA_DIRECTIVE,
            S3X_AMZ_CONTENT_SHA256,
            S3X_AMZ_STORAGE_CLASS,
            S3_LOCATION_CONSTRAINT,
            S3_PREFIX,
            S3_DELIMITER,
            S3_VERSION_ID,
            S3_MAX_KEYS,
            S3_OBJECT_LOCK_REMAINING_RETENTION_DAYS,
            S3_OBJECT_LOCK_MODE,
            S3_OBJECT_LOCK_RETAIN_UNTIL_DATE,
            S3_OBJECT_LOCK_LEGAL_HOLD,
            AWS_REFERER,
            AWS_SOURCE_IP,
            AWS_USER_AGENT,
            AWS_SECURE_TRANSPORT,
            AWS_CURRENT_TIME,
            AWS_EPOCH_TIME,
            AWS_PRINCIPAL_TYPE,
            AWS_USER_ID,
            AWS_USERNAME,
            S3_SIGNATURE_VERSION,
            S3_AUTH_TYPE,
        ];
        keys.extend(super::JWT_KEYS.iter().cloned());
        keys
    };

    // List of all common condition keys.
    pub static ref COMMON_KEYS: Vec<Key<'static>> = {
        let mut keys = vec![
            S3_SIGNATURE_VERSION,
            S3_AUTH_TYPE,
            S3X_AMZ_CONTENT_SHA256,
            S3_LOCATION_CONSTRAINT,
            AWS_REFERER,
            AWS_SOURCE_IP,
            AWS_USER_AGENT,
            AWS_SECURE_TRANSPORT,
            AWS_CURRENT_TIME,
            AWS_EPOCH_TIME,
            AWS_PRINCIPAL_TYPE,
            AWS_USER_ID,
            AWS_USERNAME,
        ];
        keys.extend(super::JWT_KEYS.iter().cloned());
        keys
    };

    // List of all admin supported keys.
    pub static ref ALL_SUPPORTED_ADMIN_KEYS: Vec<Key<'static>> = vec![
        AWS_REFERER,
        AWS_SOURCE_IP,
        AWS_USER_AGENT,
        AWS_SECURE_TRANSPORT,
        AWS_CURRENT_TIME,
        AWS_EPOCH_TIME,
    ];
}

pub(super) fn subst_func_from_values(
    values: HashMap<String, Vec<String>>,
) -> Box<dyn Fn(&str) -> String> {
    Box::new(move |v: &str| -> String {
        for key in COMMON_KEYS.iter() {
            // Empty values are not supported for policy variables.
            if let Some(rvalues) = values.get(key.name()) {
                if !rvalues.is_empty() && !rvalues[0].is_empty() {
                    return v.replace(&key.var_name(), &rvalues[0]);
                }
            }
        }
        v.to_owned()
    })
}

impl<'a> Key<'a> {
    // Returns variable key name, such as "${aws:username}"
    pub fn var_name(&self) -> String {
        format!("${{{}}}", self.0)
    }

    pub fn name(&self) -> &str {
        let name = self.0;
        if name.starts_with("aws:") {
            name.strip_prefix("aws:").unwrap()
        } else if name.starts_with("jwt:") {
            name.strip_prefix("jwt:").unwrap()
        } else if name.starts_with("ldap:") {
            name.strip_prefix("ldap:").unwrap()
        } else if name.starts_with("s3:") {
            name.strip_prefix("s3:").unwrap()
        } else {
            name
        }
    }
}

impl<'a> Valid for Key<'a> {
    fn is_valid(&self) -> bool {
        ALL_SUPPORTED_KEYS.iter().any(|k| k == self)
    }
}

impl<'a> fmt::Display for Key<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> Serialize for Key<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        if !self.is_valid() {
            return Err(S::Error::custom(format!(
                "unknown condition key '{}'",
                self.0
            )));
        }
        serializer.serialize_newtype_struct("Key", &self.0)
    }
}

impl<'de, 'a> Deserialize<'de> for Key<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyVisitor;
        impl<'de> Visitor<'de> for KeyVisitor {
            type Value = Key<'static>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a condition key string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                ALL_SUPPORTED_KEYS
                    .iter()
                    .find(|&k| k.0 == v)
                    .cloned()
                    .ok_or(E::custom(format!("invalid condition key '{}'", v)))
            }
        }

        deserializer.deserialize_str(KeyVisitor)
    }
}

pub type KeySet<'a> = HashSet<Key<'a>>;

impl<'a> super::super::ToVec<Key<'a>> for KeySet<'a> {
    fn to_vec(&self) -> Vec<Key<'a>> {
        self.iter().cloned().collect()
    }
}

#[macro_export]
macro_rules! keyset {
    ($($e:expr),*) => {{
        let mut set = HashSet::new();
        $(
            let _ = set.insert($e);
        )*
        KeySet(set)
    }};
}

#[macro_export]
macro_rules! keyset_extend {
    ($($e:expr,)+) => { keyset_extend!($($e),+) };
    ($ks:expr, $($e:expr),*) => {{
        let mut ks = $ks;
        $(
            let _ = ks.insert($e);
        )+
        ks
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_is_valid() {
        let cases = [
            (S3X_AMZ_COPY_SOURCE, true),
            (AWS_REFERER, true),
            (Key("foo"), false),
        ];
        for (key, expected_result) in cases.iter() {
            assert_eq!(
                key.is_valid(),
                *expected_result,
                "key: '{:?}', expected: {}, got: {}",
                key,
                expected_result,
                key.is_valid()
            );
        }
    }

    #[test]
    fn test_key_serialize_json() {
        let cases: [(Key, &str, bool); 2] = [
            (S3X_AMZ_COPY_SOURCE, "\"s3:x-amz-copy-source\"", false),
            (Key("foo"), "", true),
        ];
        for (key, expected_result, expected_err) in cases.iter() {
            let result = serde_json::to_string(key);
            match result {
                Ok(result) => assert_eq!(&result, *expected_result),
                Err(_) => assert!(expected_err),
            }
        }
    }

    #[test]
    fn test_key_deserialize_json() {
        let cases: [(&str, Key, bool); 3] = [
            ("\"s3:x-amz-copy-source\"", S3X_AMZ_COPY_SOURCE, false),
            ("", Key("foo"), true),
            ("\"foo\"", Key("foo"), true),
        ];
        for (data, expected_key, expected_err) in cases.iter() {
            let result: serde_json::Result<Key> = serde_json::from_str(*data);
            match result {
                Ok(result) => assert_eq!(&result, expected_key),
                Err(_) => assert!(expected_err),
            }
        }
    }

    #[test]
    fn test_key_name() {
        let cases = [
            (S3X_AMZ_COPY_SOURCE, "x-amz-copy-source"),
            (AWS_REFERER, "Referer"),
        ];
        for (key, name) in cases.iter() {
            assert_eq!(key.name(), *name);
        }
    }

    #[test]
    fn test_keyset_to_vec() {
        let cases = [
            (KeySet::new(), Vec::<Key>::new()),
            (
                vec![S3_DELIMITER].into_iter().collect::<KeySet>(),
                vec![S3_DELIMITER],
            ),
        ];
        for (set, expected_result) in cases.iter() {
            assert_eq!(&set.to_vec(), expected_result);
        }
    }
}
