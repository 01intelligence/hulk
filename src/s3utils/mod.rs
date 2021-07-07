use anyhow::bail;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref VALID_BUCKET_NAME: Regex =
        Regex::new(r#"^[A-Za-z0-9][A-Za-z0-9\.\-_:]{1,61}[A-Za-z0-9]$"#).unwrap();
    static ref VALID_BUCKET_NAME_STRICT: Regex =
        Regex::new(r#"^[a-z0-9][a-z0-9\.\-]{1,61}[a-z0-9]$"#).unwrap();
    static ref IP_ADDRESS: Regex = Regex::new(r#"^(\d+\.){3}\d+$"#).unwrap();
}

// Checks if we have a valid input bucket name.
pub fn check_valid_bucket_name(bucket_name: &str) -> anyhow::Result<()> {
    check_bucket_name_common(bucket_name, false)
}

// Checks if we have a valid input bucket name.
// This is a stricter version.
// - http://docs.aws.amazon.com/AmazonS3/latest/dev/UsingBucket.html
pub fn check_valid_bucket_name_strict(bucket_name: &str) -> anyhow::Result<()> {
    check_bucket_name_common(bucket_name, true)
}

fn check_bucket_name_common(bucket_name: &str, strict: bool) -> anyhow::Result<()> {
    if bucket_name.trim().is_empty() {
        bail!("Bucket name cannot be empty");
    }
    if bucket_name.len() < 3 {
        bail!("Bucket name cannot be shorter than 3 characters");
    }
    if bucket_name.len() > 63 {
        bail!("Bucket name cannot be longer than 63 characters");
    }
    if IP_ADDRESS.is_match(bucket_name) {
        bail!("Bucket name cannot be an ip address");
    }
    if bucket_name.contains("..") || bucket_name.contains(".-") || bucket_name.contains("-.") {
        bail!("Bucket name contains invalid characters");
    }
    if strict && !VALID_BUCKET_NAME_STRICT.is_match(bucket_name) {
        bail!("Bucket name contains invalid characters");
    }
    if !VALID_BUCKET_NAME.is_match(bucket_name) {
        bail!("Bucket name contains invalid characters");
    }
    Ok(())
}
