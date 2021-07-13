use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum SseAlgorithm {
    #[serde(rename = "AES256")]
    Aes256,
    #[serde(rename = "aws:kms")]
    AwsKms,
}
