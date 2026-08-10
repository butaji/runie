use super::{ImageContent, VideoContent};

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
