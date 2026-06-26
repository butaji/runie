//! GitHub Gist sharing.

use crate::ChatMessage;

/// Share session messages to GitHub gist (blocking).
pub fn share_session_sync(
    messages: &[ChatMessage],
    display_name: Option<&str>,
) -> Result<String, String> {
    let token = std::env::var("GITHUB_TOKEN")
        .map_err(|_| "GITHUB_TOKEN environment variable not set".to_string())?;

    let content = crate::format_as_markdown(messages, display_name);
    let description = display_name.unwrap_or("runie session");

    let client = reqwest::blocking::Client::new();
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
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("GitHub API error {}: {}", status, body));
    }

    let json: serde_json::Value = resp.json().map_err(|e| format!("JSON error: {}", e))?;
    json.get("html_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
        .ok_or_else(|| "Could not extract gist URL from response".to_string())
}
