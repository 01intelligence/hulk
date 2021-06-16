// Conditional key which is used to fetch values for any condition.
// Refer https://docs.aws.amazon.com/IAM/latest/UserGuide/list_s3.html
// for more information about available condition keys.
pub struct Key<'a>(&'a str);

// S3X_AMZ_COPY_SOURCE - key representing x-amz-copy-source HTTP header applicable to PutObject API only.
const S3X_AMZ_COPY_SOURCE: Key = Key("s3:x-amz-copy-source");

// S3X_AMZ_SERVER_SIDE_ENCRYPTION - key representing x-amz-server-side-encryption HTTP header applicable
// to PutObject API only.
const S3X_AMZ_SERVER_SIDE_ENCRYPTION: Key = Key("s3:x-amz-server-side-encryption");

// S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM - key representing
// x-amz-server-side-encryption-customer-algorithm HTTP header applicable to PutObject API only.
const S3X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM: Key =
    Key("s3:x-amz-server-side-encryption-customer-algorithm");

// S3X_AMZ_METADATA_DIRECTIVE - key representing x-amz-metadata-directive HTTP header applicable to
// PutObject API only.
const S3X_AMZ_METADATA_DIRECTIVE: Key = Key("s3:x-amz-metadata-directive");

// S3X_AMZ_CONTENT_SHA256 - set a static content-sha256 for all calls for a given action.
const S3X_AMZ_CONTENT_SHA256: Key = Key("s3:x-amz-content-sha256");

// S3X_AMZ_STORAGE_CLASS - key representing x-amz-storage-class HTTP header applicable to PutObject API
// only.
const S3X_AMZ_STORAGE_CLASS: Key = Key("s3:x-amz-storage-class");

// S3_LOCATION_CONSTRAINT - key representing LocationConstraint XML tag of CreateBucket API only.
const S3_LOCATION_CONSTRAINT: Key = Key("s3:LocationConstraint");

// S3_PREFIX - key representing prefix query parameter of ListBucket API only.
const S3_PREFIX: Key = Key("s3:prefix");

// S3_DELIMITER - key representing delimiter query parameter of ListBucket API only.
const S3_DELIMITER: Key = Key("s3:delimiter");

// S3_VERSION_ID - Enables you to limit the permission for the
// s3:PutObjectVersionTagging action to a specific object version.
const S3_VERSION_ID: Key = Key("s3:versionid");

// S3_MAX_KEYS - key representing max-keys query parameter of ListBucket API only.
const S3_MAX_KEYS: Key = Key("s3:max-keys");

// S3_OBJECT_LOCK_REMAINING_RETENTION_DAYS - key representing object-lock-remaining-retention-days
// Enables enforcement of an object relative to the remaining retention days, you can set
// minimum and maximum allowable retention periods for a bucket using a bucket policy.
// This key are specific for s3:PutObjectRetention API.
const S3_OBJECT_LOCK_REMAINING_RETENTION_DAYS: Key = Key("s3:object-lock-remaining-retention-days");

// S3_OBJECT_LOCK_MODE - key representing object-lock-mode
// Enables enforcement of the specified object retention mode
const S3_OBJECT_LOCK_MODE: Key = Key("s3:object-lock-mode");

// S3_OBJECT_LOCK_RETAIN_UNTIL_DATE - key representing object-lock-retain-util-date
// Enables enforcement of a specific retain-until-date
const S3_OBJECT_LOCK_RETAIN_UNTIL_DATE: Key = Key("s3:object-lock-retain-until-date");

// S3_OBJECT_LOCK_LEGAL_HOLD - key representing object-local-legal-hold
// Enables enforcement of the specified object legal hold status
const S3_OBJECT_LOCK_LEGAL_HOLD: Key = Key("s3:object-lock-legal-hold");

// AWS_REFERER - key representing Referer header of any API.
const AWS_REFERER: Key = Key("aws:Referer");

// AWS_SOURCE_IP - key representing client's IP address (not intermittent proxies) of any API.
const AWS_SOURCE_IP: Key = Key("aws:SourceIp");

// AWS_USER_AGENT - key representing UserAgent header for any API.
const AWS_USER_AGENT: Key = Key("aws:UserAgent");

// AWS_SECURE_TRANSPORT - key representing if the clients request is authenticated or not.
const AWS_SECURE_TRANSPORT: Key = Key("aws:SecureTransport");

// AWS_CURRENT_TIME - key representing the current time.
const AWS_CURRENT_TIME: Key = Key("aws:CurrentTime");

// AWS_EPOCH_TIME - key representing the current epoch time.
const AWS_EPOCH_TIME: Key = Key("aws:EpochTime");

// AWS_PRINCIPAL_TYPE - user principal type currently supported values are "User" and "Anonymous".
const AWS_PRINCIPAL_TYPE: Key = Key("aws:principaltype");

// AWS_USER_ID - user unique ID, in MinIO this value is same as your user Access Key.
const AWS_USER_ID: Key = Key("aws:userid");

// AWS_USERNAME - user friendly name, in MinIO this value is same as your user Access Key.
const AWS_USERNAME: Key = Key("aws:username");

// S3_SIGNATURE_VERSION - identifies the version of AWS Signature that you want to support for authenticated requests.
const S3_SIGNATURE_VERSION: Key = Key("s3:signatureversion");

// S3_AUTH_TYPE - optionally use this condition key to restrict incoming requests to use a specific authentication method.
const S3_AUTH_TYPE: Key = Key("s3:authType");
