use super::*;

/// Single content block on a user message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserContent {
    Text {
        text: String,
    },
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    ImageUrl {
        url: String,
    },
    Video {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    VideoUrl {
        url: String,
    },
    Audio {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    AudioUrl {
        url: String,
    },
}
