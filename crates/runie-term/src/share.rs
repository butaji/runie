//! GitHub Gist sharing for sessions.

use runie_core::ChatMessage;

/// Upload session content as a secret GitHub gist.
///
/// Requires `GITHUB_TOKEN` environment variable.
pub async fn share_session(
    messages: &[ChatMessage],
    display_name: Option<&str>,
) -> anyhow::Result<String> {
    let token = std::env::var("GITHUB_TOKEN")
        .map_err(|_| anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))?;

    let content = runie_core::format_as_markdown(messages, display_name);
    let description = display_name.unwrap_or("runie session");

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.github.com/gists")
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "runie")
        .json(&serde_json::json!({
            "description": description,
            "public": false,
            "files": {
                "session.md": { "content": content }
            }
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("GitHub API error {}: {}", status, body));
    }

    let json: serde_json::Value = resp.json().await?;
    parse_gist_url(&json).ok_or_else(|| anyhow::anyhow!("Could not extract gist URL from response"))
}

/// Extract the HTML URL from a GitHub gist creation response.
pub fn parse_gist_url(json: &serde_json::Value) -> Option<String> {
    json.get("html_url")?.as_str().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_url_extracted() {
        let json = serde_json::json!({
            "html_url": "https://gist.github.com/user/abc123"
        });
        assert_eq!(
            parse_gist_url(&json),
            Some("https://gist.github.com/user/abc123".to_string())
        );
    }

    #[test]
    fn share_url_missing_returns_none() {
        let json = serde_json::json!({ "id": "abc123" });
        assert_eq!(parse_gist_url(&json), None);
    }
}
