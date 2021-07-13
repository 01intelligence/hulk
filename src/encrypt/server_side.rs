use const_format::concatcp;
use md5::Digest;
use reqwest::header::HeaderMap;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use strum::Display;

// The AWS SSE header used for SSE-S3 and SSE-KMS.
const SSE_GENERIC_HEADER: &str = "X-Amz-Server-Side-Encryption";

// The AWS SSE-KMS key id.
const SSE_KMS_KEY_ID: &str = concatcp!(SSE_GENERIC_HEADER, "-Aws-Kms-Key-Id");
// The AWS SSE-KMS Encryption Context data.
const SSE_ENCRYPTION_CONTEXT: &str = concatcp!(SSE_GENERIC_HEADER, "-Context");

// The AWS SSE-C algorithm HTTP header key.
const SSE_CUSTOMER_ALGORITHM: &str = concatcp!(SSE_GENERIC_HEADER, "-Customer-Algorithm");
// The AWS SSE-C encryption key HTTP header key.
const SSE_CUSTOMER_KEY: &str = concatcp!(SSE_GENERIC_HEADER, "-Customer-Key");
// The AWS SSE-C encryption key MD5 HTTP header key.
const SSE_CUSTOMER_KEY_MD5: &str = concatcp!(SSE_GENERIC_HEADER, "-Customer-Key-MD5");

// The AWS SSE-C algorithm HTTP header key for CopyObject API.
const SSE_COPY_CUSTOMER_ALGORITHM: &str =
    "X-Amz-Copy-Source-Server-Side-Encryption-Customer-Algorithm";
// The AWS SSE-C encryption key HTTP header key for CopyObject API.
const SSE_COPY_CUSTOMER_KEY: &str = "X-Amz-Copy-Source-Server-Side-Encryption-Customer-Key";
// The AWS SSE-C encryption key MD5 HTTP header key for CopyObject API.
const SSE_COPY_CUSTOMER_KEY_MD5: &str = "X-Amz-Copy-Source-Server-Side-Encryption-Customer-Key-MD5";

// The server-side-encryption method.
#[derive(Serialize, Deserialize, Display)]
pub enum Type {
    // Server-side-encryption with customer provided keys
    #[serde(rename = "SSE-C")]
    #[strum(serialize = "SSE-C")]
    Ssec,
    // Server-side-encryption with managed keys
    #[serde(rename = "KMS")]
    #[strum(serialize = "KMS")]
    Kms,
    // Server-side-encryption using S3 storage encryption
    #[serde(rename = "S3")]
    #[strum(serialize = "S3")]
    S3,
}

// Server-side-encryption.
pub enum ServerSide {
    Ssec([u8; 32]),
    SsecCopy([u8; 32]),
    Kms {
        key: String,
        context: Option<String>,
    },
    S3,
}

impl ServerSide {
    pub fn sse() -> Self {
        ServerSide::S3
    }

    pub fn sse_kms(key: String, context: Option<String>) -> Self {
        ServerSide::Kms { key, context }
    }

    pub fn ssec(key: [u8; 32]) -> Self {
        ServerSide::Ssec(key)
    }

    pub fn unchanged_or_from_ssec_copy(self) -> Self {
        match self {
            ServerSide::SsecCopy(ssec) => ServerSide::Ssec(ssec),
            s => s,
        }
    }

    pub fn unchanged_or_from_ssec(self) -> Self {
        match self {
            ServerSide::Ssec(ssec) => ServerSide::SsecCopy(ssec),
            s => s,
        }
    }

    // Returns the server-side-encryption method.
    fn typ(&self) -> Type {
        match self {
            ServerSide::Ssec(_) => Type::Ssec,
            ServerSide::SsecCopy(_) => Type::Ssec,
            ServerSide::Kms { .. } => Type::Kms,
            ServerSide::S3 => Type::S3,
        }
    }

    // Adds encryption headers to the provided HTTP request.
    // It marks an HTTP request as server-side-encryption request
    // and inserts the required data into the headers.
    fn marshal(&self, req: RequestBuilder) -> RequestBuilder {
        match self {
            ServerSide::Ssec(ssec) => {
                let key_md5 = md5::Md5::digest(ssec).to_vec();
                let mut headers = HeaderMap::new();
                headers.insert(SSE_CUSTOMER_ALGORITHM, "AES256".parse().unwrap());
                headers.insert(SSE_CUSTOMER_KEY, base64::encode(ssec).parse().unwrap());
                headers.insert(
                    SSE_CUSTOMER_KEY_MD5,
                    base64::encode(key_md5).parse().unwrap(),
                );
                req.headers(headers)
            }
            ServerSide::SsecCopy(ssec) => {
                let key_md5 = md5::Md5::digest(ssec).to_vec();
                let mut headers = HeaderMap::new();
                headers.insert(SSE_COPY_CUSTOMER_ALGORITHM, "AES256".parse().unwrap());
                headers.insert(SSE_COPY_CUSTOMER_KEY, base64::encode(ssec).parse().unwrap());
                headers.insert(
                    SSE_COPY_CUSTOMER_KEY_MD5,
                    base64::encode(key_md5).parse().unwrap(),
                );
                req.headers(headers)
            }
            ServerSide::Kms { key, context } => {
                let mut headers = HeaderMap::new();
                headers.insert(SSE_GENERIC_HEADER, "aws:kms".parse().unwrap());
                if !key.is_empty() {
                    headers.insert(SSE_KMS_KEY_ID, key.parse().unwrap());
                }
                if let Some(context) = context {
                    headers.insert(
                        SSE_ENCRYPTION_CONTEXT,
                        base64::encode(context).parse().unwrap(),
                    );
                }
                req.headers(headers)
            }
            ServerSide::S3 => req.header(SSE_GENERIC_HEADER, "AES256"),
        }
    }
}
