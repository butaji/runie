# Share as GitHub Gist (/share)

**Status**: todo
**Milestone**: R3
**Category**: Sessions

## Description

Upload current session as a secret GitHub gist.

## Architecture

```rust
fn cmd_share(_args: &str) -> Option<Event> {
    Some(Event::ShareSession)
}

async fn share_session(session: &Session) -> Result<String> {
    let content = format_session_for_gist(session);
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.github.com/gists")
        .header("Authorization", format!("token {}", token))
        .json(&json!({
            "description": format!("{} session", session.name),
            "public": false,
            "files": {
                "session.md": { "content": content }
            }
        }))
        .send()
        .await?;
    let json: serde_json::Value = resp.json().await?;
    Ok(json["html_url"].as_str().unwrap_or("").to_string())
}
```

## Acceptance Criteria

- [ ] `/share` uploads session as secret GitHub gist
- [ ] Requires `GITHUB_TOKEN` env var or config
- [ ] Shows gist URL in chat
- [ ] Formats session as markdown
- [ ] Error if no token configured

## Tests

### Layer 1
- [ ] `format_session_markdown` — output is valid markdown
- [ ] `share_url_extracted` — parses gist URL from response
