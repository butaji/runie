use super::ImageContent;

impl ImageContent {
    pub fn new(mime_type: impl Into<String>, data: impl Into<String>) -> Result<Self, String> {
        let mime_type = mime_type.into();
        let data = data.into();
        if !mime_type.starts_with("image/") {
            return Err(format!("unsupported image MIME type: {mime_type}"));
        }
        if !is_base64_payload(&data) {
            return Err("image data must be non-empty base64".into());
        }
        Ok(Self { data, mime_type })
    }
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
