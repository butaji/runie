use serde::{Deserialize, Serialize};

/// Image content block, preserved as provider-neutral MIME and base64 data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageContent {
    pub data: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

/// Video content block, preserved until a provider adapter chooses its wire form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoContent {
    pub data: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}
