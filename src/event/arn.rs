use std::fmt;

use anyhow::ensure;

pub use super::*;

// SQS resource name representation.
pub struct Arn {
    pub target_id: TargetId,
    pub(super) region: String,
}

impl fmt::Display for Arn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.target_id.id.is_empty() && self.target_id.name.is_empty() && self.region.is_empty()
        {
            return write!(f, "");
        }
        write!(f, "arn:hulk:sqs:{}:{}", self.region, self.target_id)
    }
}

fn parse_arn(s: &str) -> anyhow::Result<Arn> {
    // ARN must be in the format of arn:hulk:sqs:<REGION>:<ID>:<TYPE>
    ensure!(
        s.starts_with("arn:hulk:sqs:"),
        EventError::InvalidArn(s.to_owned())
    );
    let tokens: Vec<_> = s.split(':').collect();
    ensure!(
        tokens.len() == 6 && !tokens[4].is_empty() && !tokens[5].is_empty(),
        EventError::InvalidArn(s.to_owned())
    );
    Ok(Arn {
        target_id: TargetId {
            id: tokens[4].to_owned(),
            name: tokens[5].to_owned(),
        },
        region: tokens[3].to_owned(),
    })
}
