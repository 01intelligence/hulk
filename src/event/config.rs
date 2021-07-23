use serde::{Deserialize, Serialize};

use super::*;

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub queue_configuration: Vec<Queue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cloud_function_configuration: Vec<CloudFunction>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topic_configuration: Vec<Topic>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Queue {
    pub id: String,
    pub filter: Filter,
    pub event: Vec<Name>,
    pub queue: Arn,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Filter {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub s3_key: Vec<FilterRule>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct FilterRule {
    pub filter_rule: FilterRuleInner,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct FilterRuleInner {
    pub name: String,
    pub value: String,
}

// Unused, but available for completion.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct CloudFunction {
    pub cloud_function: String,
}

// Unused, but available for completion.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Topic {
    pub topic: String,
}

#[cfg(test)]
mod tests {
    use super::*;
}
