use super::{ImageContent, UserContent, VideoContent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaWireFormat {
    Pi,
    OpenAiChat,
    OpenAiResponses,
}

/// Encode validated user media at the provider boundary.
pub fn encode_user_content(
    content: &UserContent,
    format: MediaWireFormat,
) -> Result<serde_json::Value, String> {
    match (content, format) {
        (UserContent::Text { text }, _) => Ok(serde_json::json!({"type": "text", "text": text})),
        (UserContent::Image { data, mime_type }, MediaWireFormat::Pi) => {
            Ok(serde_json::json!({"type":"image","data":data,"mimeType":mime_type}))
        }
        (UserContent::Image { data, mime_type }, MediaWireFormat::OpenAiChat) => Ok(
            serde_json::json!({"type":"image_url","image_url":{"url":format!("data:{mime_type};base64,{data}")}}),
        ),
        (UserContent::Image { data, mime_type }, MediaWireFormat::OpenAiResponses) => Ok(
            serde_json::json!({"type":"input_image","image_url":format!("data:{mime_type};base64,{data}")}),
        ),
        (UserContent::Video { .. }, MediaWireFormat::Pi) => serde_json::to_value(content)
            .map_err(|error| format!("encode Pi video content: {error}")),
        (UserContent::Video { .. }, _) => {
            Err("selected provider wire format does not support video content".into())
        }
    }
}

impl ImageContent {
    pub fn new(mime_type: impl Into<String>, data: impl Into<String>) -> Result<Self, String> {
        let mime_type = mime_type.into();
        let data = data.into();
        validate_media("image", &mime_type, &data)?;
        Ok(Self { data, mime_type })
    }
}

impl VideoContent {
    pub fn new(mime_type: impl Into<String>, data: impl Into<String>) -> Result<Self, String> {
        let mime_type = mime_type.into();
        let data = data.into();
        validate_media("video", &mime_type, &data)?;
        Ok(Self { data, mime_type })
    }
}

fn validate_media(kind: &str, mime_type: &str, data: &str) -> Result<(), String> {
    if !mime_type.starts_with(&format!("{kind}/")) {
        return Err(format!("unsupported {kind} MIME type: {mime_type}"));
    }
    if !is_base64_payload(data) {
        return Err(format!("{kind} data must be non-empty base64"));
    }
    Ok(())
}

fn is_base64_payload(data: &str) -> bool {
    !data.is_empty()
        && data.len().is_multiple_of(4)
        && data.chars().enumerate().all(|(index, character)| {
            character.is_ascii_alphanumeric()
                || matches!(character, '+' | '/')
                || (character == '=' && index >= data.len().saturating_sub(2))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_encoding_is_explicit_per_provider_wire_format() {
        let content = UserContent::Image {
            data: "aGVsbG8=".into(),
            mime_type: "image/png".into(),
        };
        let chat = encode_user_content(&content, MediaWireFormat::OpenAiChat).unwrap();
        assert_eq!(chat["type"], "image_url");
        assert_eq!(chat["image_url"]["url"], "data:image/png;base64,aGVsbG8=");
        let responses = encode_user_content(&content, MediaWireFormat::OpenAiResponses).unwrap();
        assert_eq!(responses["type"], "input_image");
        assert_eq!(responses["image_url"], "data:image/png;base64,aGVsbG8=");
    }

    #[test]
    fn unsupported_provider_video_encoding_is_rejected() {
        let content = UserContent::Video {
            data: "aGVsbG8=".into(),
            mime_type: "video/mp4".into(),
        };
        assert!(encode_user_content(&content, MediaWireFormat::OpenAiChat).is_err());
        assert_eq!(
            encode_user_content(&content, MediaWireFormat::Pi).unwrap()["type"],
            "video"
        );
    }
}
