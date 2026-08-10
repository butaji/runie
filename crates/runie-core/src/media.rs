use super::{ImageContent, UserContent, VideoContent};

const MAX_MEDIA_BASE64_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaWireFormat {
    Pi,
    OpenAiChat,
    OpenAiResponses,
    Gemini,
    Anthropic,
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
        (UserContent::Image { data, mime_type }, MediaWireFormat::Gemini)
        | (UserContent::Video { data, mime_type }, MediaWireFormat::Gemini) => {
            Ok(serde_json::json!({"inline_data":{"mime_type":mime_type,"data":data}}))
        }
        (UserContent::Image { data, mime_type }, MediaWireFormat::Anthropic) => Ok(
            serde_json::json!({"type":"image","source":{"type":"base64","media_type":mime_type,"data":data}}),
        ),
        (UserContent::Video { .. }, MediaWireFormat::Pi) => serde_json::to_value(content)
            .map_err(|error| format!("encode Pi video content: {error}")),
        (UserContent::Video { .. }, _) => {
            Err("selected provider wire format does not support video content".into())
        }
    }
}

/// Encode an entire user turn as an ordered provider payload. Keeping the
/// sequence intact lets adapters handle mixed text and media without
/// reconstructing content blocks themselves.
pub fn encode_user_contents(
    contents: &[UserContent],
    format: MediaWireFormat,
) -> Result<Vec<serde_json::Value>, String> {
    contents
        .iter()
        .map(|content| encode_user_content(content, format))
        .collect()
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
    if data.len() > MAX_MEDIA_BASE64_BYTES {
        return Err(format!(
            "{kind} data exceeds {MAX_MEDIA_BASE64_BYTES} encoded bytes"
        ));
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
        assert_eq!(
            encode_user_content(&content, MediaWireFormat::Gemini).unwrap()["inline_data"],
            serde_json::json!({"mime_type":"video/mp4","data":"aGVsbG8="})
        );
    }

    #[test]
    fn gemini_media_encoding_keeps_mixed_content_order() {
        let contents = vec![
            UserContent::Text {
                text: "look".into(),
            },
            UserContent::Image {
                data: "aGVsbG8=".into(),
                mime_type: "image/png".into(),
            },
        ];
        let encoded = encode_user_contents(&contents, MediaWireFormat::Gemini).unwrap();
        assert_eq!(encoded[0]["type"], "text");
        assert_eq!(encoded[1]["inline_data"]["mime_type"], "image/png");
    }

    #[test]
    fn anthropic_image_encoding_uses_native_base64_source() {
        let content = UserContent::Image {
            data: "aGVsbG8=".into(),
            mime_type: "image/png".into(),
        };
        let encoded = encode_user_content(&content, MediaWireFormat::Anthropic).unwrap();
        assert_eq!(encoded["type"], "image");
        assert_eq!(encoded["source"]["type"], "base64");
        assert_eq!(encoded["source"]["media_type"], "image/png");
    }

    #[test]
    fn anthropic_video_encoding_remains_explicitly_unsupported() {
        let content = UserContent::Video {
            data: "aGVsbG8=".into(),
            mime_type: "video/mp4".into(),
        };
        assert!(encode_user_content(&content, MediaWireFormat::Anthropic).is_err());
    }

    #[test]
    fn mixed_user_turn_encoding_preserves_content_order() {
        let contents = vec![
            UserContent::Text {
                text: "look".into(),
            },
            UserContent::Image {
                data: "aGVsbG8=".into(),
                mime_type: "image/png".into(),
            },
        ];
        let encoded = encode_user_contents(&contents, MediaWireFormat::OpenAiResponses).unwrap();
        assert_eq!(encoded[0]["type"], "text");
        assert_eq!(encoded[1]["type"], "input_image");
    }

    #[test]
    fn media_constructor_rejects_unbounded_payloads() {
        let data = "A".repeat(MAX_MEDIA_BASE64_BYTES + 4);
        assert!(ImageContent::new("image/png", data).is_err());
    }
}
