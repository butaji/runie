pub(super) fn gemini_media_url(url: &str, fallback_mime: &str) -> serde_json::Value {
    if let Some((meta, data)) = url
        .strip_prefix("data:")
        .and_then(|value| value.split_once(','))
    {
        let mime = meta
            .split(';')
            .next()
            .filter(|value| !value.is_empty())
            .unwrap_or(fallback_mime);
        serde_json::json!({"inline_data":{"mime_type":mime,"data":data}})
    } else {
        serde_json::json!({"file_data":{"file_uri":url,"mime_type":fallback_mime}})
    }
}

#[cfg(test)]
mod tests {
    use super::gemini_media_url;

    #[test]
    fn projects_remote_and_inline_media_as_gemini_data() {
        assert_eq!(
            gemini_media_url("https://example.test/a.mp3", "audio/mpeg")["file_data"]["mime_type"],
            "audio/mpeg"
        );
        assert_eq!(
            gemini_media_url("data:video/mp4;base64,AAAA", "video/mp4")["inline_data"]["mime_type"],
            "video/mp4"
        );
    }
}
