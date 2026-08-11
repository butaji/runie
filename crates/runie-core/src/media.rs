use super::{AudioContent, ImageContent, UserContent, VideoContent};
#[path = "media_url.rs"]
mod media_url;
use media_url::gemini_media_url;

const MAX_MEDIA_BASE64_BYTES: usize = 16 * 1024 * 1024;

macro_rules! media_wire_formats {
    ($(($variant:ident, $wire:literal, $video:literal, $audio:literal)),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "kebab-case")]
        pub enum MediaWireFormat {
            $($variant),+
        }

        impl MediaWireFormat {
            pub const fn wire_name(self) -> &'static str {
                match self { $(Self::$variant => $wire),+ }
            }

            pub fn from_wire_name(name: &str) -> Option<Self> {
                match name { $($wire => Some(Self::$variant),)+ _ => None }
            }

            pub const fn supports_video(self) -> bool {
                match self { $(Self::$variant => $video),+ }
            }

            pub const fn supports_audio(self) -> bool {
                match self { $(Self::$variant => $audio),+ }
            }
        }
    };
}
media_wire_formats! {
    (Pi, "pi", true, true),
    (OpenAiChat, "openai-chat", true, true),
    (OpenAiResponses, "openai-responses", false, true),
    (Gemini, "gemini", true, true),
    (Anthropic, "anthropic", true, false),
}
/// Encode validated user media at the provider boundary.
pub fn encode_user_content(
    content: &UserContent,
    format: MediaWireFormat,
) -> Result<serde_json::Value, String> {
    match content {
        UserContent::Text { text } => Ok(serde_json::json!({"type":"text","text":text})),
        UserContent::Image { data, mime_type } => encode_image(data, mime_type, format),
        UserContent::ImageUrl { url } => encode_image_url(url, format),
        UserContent::Video { data, mime_type } => encode_video(data, mime_type, format),
        UserContent::VideoUrl { url } => encode_video_url(url, format),
        UserContent::Audio { data, mime_type } => encode_audio(data, mime_type, format),
        UserContent::AudioUrl { url } => encode_audio_url(url, format),
    }
}
fn unsupported_media(format: MediaWireFormat) -> Result<serde_json::Value, String> {
    Err(format!(
        "selected provider wire format does not support media: {}",
        format.wire_name()
    ))
}
#[rustfmt::skip]
fn omitted_media(kind: &str) -> serde_json::Value { serde_json::json!({"type":"text","text":format!("({kind} omitted: not supported by this provider)")}) }
fn encode_image(
    data: &str,
    mime_type: &str,
    format: MediaWireFormat,
) -> Result<serde_json::Value, String> {
    match format {
        MediaWireFormat::Pi => {
            Ok(serde_json::json!({"type":"image","data":data,"mimeType":mime_type}))
        }
        MediaWireFormat::OpenAiChat => Ok(
            serde_json::json!({"type":"image_url","image_url":{"url":format!("data:{mime_type};base64,{data}")}}),
        ),
        MediaWireFormat::OpenAiResponses => Ok(serde_json::json!({
            "type":"input_image",
            "detail":"auto",
            "image_url":format!("data:{mime_type};base64,{data}"),
        })),
        MediaWireFormat::Gemini => {
            Ok(serde_json::json!({"inline_data":{"mime_type":mime_type,"data":data}}))
        }
        MediaWireFormat::Anthropic => Ok(
            serde_json::json!({"type":"image","source":{"type":"base64","media_type":mime_type,"data":data}}),
        ),
    }
}
fn encode_video(
    data: &str,
    mime_type: &str,
    format: MediaWireFormat,
) -> Result<serde_json::Value, String> {
    match format {
        MediaWireFormat::Pi => encode_pi_media(&UserContent::Video {
            data: data.into(),
            mime_type: mime_type.into(),
        }),
        MediaWireFormat::OpenAiChat => Ok(
            serde_json::json!({"type":"video_url","video_url":{"url":format!("data:{mime_type};base64,{data}")}}),
        ),
        MediaWireFormat::Gemini => {
            Ok(serde_json::json!({"inline_data":{"mime_type":mime_type,"data":data}}))
        }
        MediaWireFormat::Anthropic => Ok(
            serde_json::json!({"type":"video","source":{"type":"base64","media_type":mime_type,"data":data}}),
        ),
        MediaWireFormat::OpenAiResponses => Ok(omitted_media("video")),
    }
}
fn encode_image_url(url: &str, format: MediaWireFormat) -> Result<serde_json::Value, String> {
    validate_media_url(url)?;
    match format {
        MediaWireFormat::Pi => encode_pi_media(&UserContent::ImageUrl { url: url.into() }),
        MediaWireFormat::OpenAiChat => {
            Ok(serde_json::json!({"type":"image_url","image_url":{"url":url}}))
        }
        MediaWireFormat::OpenAiResponses => {
            Ok(serde_json::json!({"type":"input_image","detail":"auto","image_url":url}))
        }
        MediaWireFormat::Anthropic => Ok(serde_json::json!({
            "type":"image",
            "source":{"type":"url","url":url}
        })),
        _ => unsupported_media(format),
    }
}
fn encode_video_url(url: &str, format: MediaWireFormat) -> Result<serde_json::Value, String> {
    validate_media_url(url)?;
    match format {
        MediaWireFormat::Pi => encode_pi_media(&UserContent::VideoUrl { url: url.into() }),
        MediaWireFormat::OpenAiChat => {
            Ok(serde_json::json!({"type":"video_url","video_url":{"url":url}}))
        }
        MediaWireFormat::OpenAiResponses => Ok(omitted_media("video")),
        MediaWireFormat::Gemini => Ok(gemini_media_url(url, "video/mp4")),
        _ => unsupported_media(format),
    }
}
fn encode_audio_url(url: &str, format: MediaWireFormat) -> Result<serde_json::Value, String> {
    validate_media_url(url)?;
    match format {
        MediaWireFormat::Pi => encode_pi_media(&UserContent::AudioUrl { url: url.into() }),
        MediaWireFormat::OpenAiChat => {
            Ok(serde_json::json!({"type":"audio_url","audio_url":{"url":url}}))
        }
        MediaWireFormat::OpenAiResponses => {
            Ok(serde_json::json!({"type":"input_file","file_url":url}))
        }
        MediaWireFormat::Anthropic => Ok(omitted_media("audio")),
        MediaWireFormat::Gemini => Ok(gemini_media_url(url, "audio/mpeg")),
    }
}
fn validate_media_url(url: &str) -> Result<(), String> {
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("data:") {
        Ok(())
    } else {
        Err("media URL must use http, https, or data scheme".into())
    }
}
fn encode_audio(
    data: &str,
    mime_type: &str,
    format: MediaWireFormat,
) -> Result<serde_json::Value, String> {
    match format {
        MediaWireFormat::Pi => encode_pi_media(&UserContent::Audio {
            data: data.into(),
            mime_type: mime_type.into(),
        }),
        MediaWireFormat::OpenAiChat => Ok(
            serde_json::json!({"type":"audio_url","audio_url":{"url":format!("data:{mime_type};base64,{data}")}}),
        ),
        MediaWireFormat::OpenAiResponses => encode_responses_audio(data, mime_type),
        MediaWireFormat::Gemini => {
            Ok(serde_json::json!({"inline_data":{"mime_type":mime_type,"data":data}}))
        }
        MediaWireFormat::Anthropic => Ok(omitted_media("audio")),
    }
}
fn encode_pi_media(content: &UserContent) -> Result<serde_json::Value, String> {
    serde_json::to_value(content).map_err(|error| format!("encode Pi media content: {error}"))
}
fn encode_responses_audio(data: &str, mime_type: &str) -> Result<serde_json::Value, String> {
    let filename = match mime_type {
        "audio/mp3" | "audio/mpeg" => "inline.mp3",
        "audio/wav" | "audio/x-wav" => "inline.wav",
        _ => {
            return Err(format!(
                "unsupported Responses audio MIME type: {mime_type}"
            ))
        }
    };
    Ok(serde_json::json!({"type":"input_file","file_data":data,"filename":filename}))
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
impl AudioContent {
    pub fn new(mime_type: impl Into<String>, data: impl Into<String>) -> Result<Self, String> {
        let mime_type = mime_type.into();
        let data = data.into();
        validate_media("audio", &mime_type, &data)?;
        Ok(Self { data, mime_type })
    }
}
fn validate_media(kind: &str, mime_type: &str, data: &str) -> Result<(), String> {
    let valid_mime = mime_type
        .strip_prefix(&format!("{kind}/"))
        .is_some_and(|subtype| !subtype.is_empty() && subtype.chars().all(|c| !c.is_whitespace()));
    if !valid_mime {
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
        assert_eq!(responses["detail"], "auto");
    }
    #[test]
    fn url_media_preserves_provider_native_sources() {
        let image = UserContent::ImageUrl {
            url: "https://example.test/image.png".into(),
        };
        let audio = UserContent::AudioUrl {
            url: "https://example.test/audio.mp3".into(),
        };
        let video = UserContent::VideoUrl {
            url: "https://example.test/video.mp4".into(),
        };

        let image_payload = encode_user_content(&image, MediaWireFormat::OpenAiResponses).unwrap();
        assert_eq!(image_payload["detail"], "auto");
        assert_eq!(image_payload["image_url"], "https://example.test/image.png");
        let audio_payload = encode_user_content(&audio, MediaWireFormat::OpenAiResponses).unwrap();
        assert_eq!(audio_payload["file_url"], "https://example.test/audio.mp3");
        assert_eq!(
            encode_user_content(&audio, MediaWireFormat::Anthropic).unwrap()["text"],
            "(audio omitted: not supported by this provider)"
        );
        let video_payload = encode_user_content(&video, MediaWireFormat::OpenAiChat).unwrap();
        assert_eq!(
            video_payload["video_url"]["url"],
            "https://example.test/video.mp4"
        );
        assert_eq!(
            encode_user_content(&video, MediaWireFormat::OpenAiResponses).unwrap()["text"],
            "(video omitted: not supported by this provider)"
        );
        assert!(encode_user_content(
            &UserContent::ImageUrl {
                url: "file:///tmp/image.png".into(),
            },
            MediaWireFormat::OpenAiChat,
        )
        .is_err());
    }
    #[test]
    fn pi_url_media_preserves_shared_content_shape() {
        let cases = [
            UserContent::ImageUrl {
                url: "https://example.test/image.png".into(),
            },
            UserContent::VideoUrl {
                url: "https://example.test/video.mp4".into(),
            },
            UserContent::AudioUrl {
                url: "https://example.test/audio.mp3".into(),
            },
        ];

        for content in cases {
            let encoded = encode_user_content(&content, MediaWireFormat::Pi).unwrap();
            assert_eq!(encoded, serde_json::to_value(content).unwrap());
        }
    }
    #[test]
    fn openai_chat_video_encoding_uses_video_url_data() {
        let content = UserContent::Video {
            data: "aGVsbG8=".into(),
            mime_type: "video/mp4".into(),
        };
        let chat = encode_user_content(&content, MediaWireFormat::OpenAiChat).unwrap();
        assert_eq!(chat["type"], "video_url");
        assert_eq!(chat["video_url"]["url"], "data:video/mp4;base64,aGVsbG8=");
        assert_eq!(
            encode_user_content(&content, MediaWireFormat::Pi).unwrap()["type"],
            "video"
        );
        assert_eq!(
            encode_user_content(&content, MediaWireFormat::Gemini).unwrap()["inline_data"],
            serde_json::json!({"mime_type":"video/mp4","data":"aGVsbG8="})
        );
        assert!(MediaWireFormat::Gemini.supports_video());
        assert!(MediaWireFormat::OpenAiChat.supports_video());
        assert_eq!(
            MediaWireFormat::OpenAiResponses.wire_name(),
            "openai-responses"
        );
    }
    #[test]
    fn gemini_audio_encoding_uses_inline_data() {
        let content = UserContent::Audio {
            data: "aGVsbG8=".into(),
            mime_type: "audio/mpeg".into(),
        };
        let encoded = encode_user_content(&content, MediaWireFormat::Gemini).unwrap();
        assert_eq!(encoded["inline_data"]["mime_type"], "audio/mpeg");
        assert_eq!(encoded["inline_data"]["data"], "aGVsbG8=");
    }
    #[test]
    fn audio_constructor_shares_the_validated_media_boundary() {
        let audio = AudioContent::new("audio/mpeg", "aGVsbG8=").unwrap();
        assert_eq!(audio.mime_type, "audio/mpeg");
        assert!(AudioContent::new("image/png", "aGVsbG8=").is_err());
        assert!(AudioContent::new("audio/", "aGVsbG8=").is_err());
        assert!(AudioContent::new("audio/mpeg", "not-base64").is_err());
    }
    #[test]
    fn media_wire_formats_round_trip_as_replay_data() {
        for format in [
            MediaWireFormat::Pi,
            MediaWireFormat::OpenAiChat,
            MediaWireFormat::OpenAiResponses,
            MediaWireFormat::Gemini,
            MediaWireFormat::Anthropic,
        ] {
            let encoded = serde_json::to_string(&format).unwrap();
            let decoded: MediaWireFormat = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, format);
            assert_eq!(
                MediaWireFormat::from_wire_name(format.wire_name()),
                Some(format)
            );
        }
        assert_eq!(MediaWireFormat::from_wire_name("unknown"), None);
    }
    #[test]
    fn openai_chat_audio_encoding_uses_audio_url_data() {
        let content = UserContent::Audio {
            data: "aGVsbG8=".into(),
            mime_type: "audio/mpeg".into(),
        };
        let encoded = encode_user_content(&content, MediaWireFormat::OpenAiChat).unwrap();
        assert_eq!(encoded["type"], "audio_url");
        assert_eq!(
            encoded["audio_url"]["url"],
            "data:audio/mpeg;base64,aGVsbG8="
        );
        assert!(MediaWireFormat::OpenAiChat.supports_audio());
    }
    #[test]
    fn openai_responses_audio_encoding_uses_inline_file_data() {
        let content = UserContent::Audio {
            data: "aGVsbG8=".into(),
            mime_type: "audio/mpeg".into(),
        };
        let encoded = encode_user_content(&content, MediaWireFormat::OpenAiResponses).unwrap();
        assert_eq!(
            encoded,
            serde_json::json!({
                "type": "input_file", "file_data": "aGVsbG8=", "filename": "inline.mp3"
            })
        );
        assert!(MediaWireFormat::OpenAiResponses.supports_audio());
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
    fn anthropic_image_url_encoding_uses_remote_url_source() {
        let content = UserContent::ImageUrl {
            url: "https://example.com/image.png".into(),
        };
        let encoded = encode_user_content(&content, MediaWireFormat::Anthropic).unwrap();
        assert_eq!(
            encoded,
            serde_json::json!({
                "type": "image",
                "source": {"type": "url", "url": "https://example.com/image.png"}
            })
        );
    }

    #[test]
    fn anthropic_video_encoding_uses_base64_source_data() {
        let content = UserContent::Video {
            data: "aGVsbG8=".into(),
            mime_type: "video/mp4".into(),
        };
        let encoded = encode_user_content(&content, MediaWireFormat::Anthropic).unwrap();
        assert_eq!(encoded["type"], "video");
        assert_eq!(encoded["source"]["media_type"], "video/mp4");
        assert!(MediaWireFormat::Anthropic.supports_video());
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
