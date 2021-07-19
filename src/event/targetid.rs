use std::fmt;

use anyhow::ensure;

use super::*;

pub struct TargetId {
    pub id: String,
    pub name: String,
}

impl fmt::Display for TargetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.id, self.name)
    }
}

impl TargetId {
    pub fn into_arn(self, region: String) -> Arn {
        Arn {
            target_id: self,
            region,
        }
    }
}

fn parse_target_id(s: &str) -> anyhow::Result<TargetId> {
    let tokens: Vec<_> = s.split(':').collect();
    ensure!(tokens.len() == 2, EventError::InvalidTargetId(s.to_owned()));
    Ok(TargetId {
        id: tokens[0].to_owned(),
        name: tokens[1].to_owned(),
    })
}
