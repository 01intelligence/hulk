use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Tag {
    /// <p>Name of the object key.</p>
    pub key: String,
    /// <p>Value of the tag.</p>
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Tagging {
    /// <p>A collection for a set of tags</p>
    pub tag_set: Vec<Tag>,
}
