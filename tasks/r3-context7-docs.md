# Context7 Documentation Fetcher

**Status**: todo
**Milestone**: R3
**Category**: Tools

## Description

Fetch LLM-friendly documentation (`llms.txt`) from [context7.com](https://context7.com) for any library or package. When the agent encounters unfamiliar dependencies, it can pull up-to-date documentation context automatically.

**Why this matters:** Context7 (by Upstash, 57k⭐) maintains structured documentation for npm packages, Rust crates, Python packages, and more. Unlike static docs, context7 provides `llms.txt` — condensed, LLM-optimized documentation that fits in context windows.

**Research:**
- ctx7 CLI (`github.com/hsbacot/ctx7`) — Go tool that queries context7.com API
- API endpoint: `https://context7.com/api/v2/libs/search?q={query}`
- Documentation URL: `https://context7.com/{library}/llms.txt`

## Architecture

```rust
pub async fn fetch_context7_docs(query: &str) -> Result<String> {
    let client = reqwest::Client::new();
    
    // 1. Search for library
    let search_url = format!("https://context7.com/api/v2/libs/search?q={}",
        urlencoding::encode(query));
    let search_resp: serde_json::Value = client.get(&search_url)
        .send().await?
        .json().await?;
    
    let library = search_resp["results"][0]["id"].as_str()
        .ok_or_else(|| anyhow!("No results found for: {}", query))?;
    
    // 2. Fetch llms.txt
    let docs_url = format!("https://context7.com/{}/llms.txt", library);
    let docs = client.get(&docs_url)
        .send().await?
        .text().await?;
    
    Ok(docs)
}
```

### Tool Registration

```rust
pub fn context7_tool() -> Tool {
    Tool {
        name: "fetch_docs".into(),
        description: "Fetch up-to-date library documentation from context7.com. \
                      Use when you need to understand an API, library, or framework \
                      that appears in the codebase.".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "library": {
                    "type": "string",
                    "description": "Library name to search for (e.g., 'tokio', 'react', 'numpy')"
                }
            },
            "required": ["library"]
        }),
    }
}
```

## Acceptance Criteria

- [ ] `fetch_docs` tool searches context7.com and returns `llms.txt` content
- [ ] Fuzzy search — partial matches work ("react" → "react-router")
- [ ] Results truncated to fit context window (50KB limit)
- [ ] Error gracefully when library not found
- [ ] Shows source URL in tool output
- [ ] Cached per-session (avoid re-fetching same library)

## Files

| File | Description |
|------|-------------|
| `crates/runie-agent/src/context7.rs` | New: API client + search + fetch |
| `crates/runie-agent/src/tools.rs` | Register `fetch_docs` tool |

## Tests

### Layer 1
- [ ] `parse_search_response` — extracts library ID from JSON
- [ ] `truncate_docs_respects_limit` — >50KB truncated
- [ ] `cache_hits_avoid_refetch` — second call returns cached

### Layer 4 — Smoke
- [ ] `context7_fetch_no_panic.sh` — tmux: agent uses fetch_docs → no panic

## Notes

- **No new dependencies needed.** Uses `reqwest` + `serde_json` (already in project).
- **Optional feature.** If context7.com is down, tool returns error message.
- **Not in pi.** This is a runie differentiator — pi has no built-in docs fetcher.
- **Complements existing tools.** Works alongside `read_file`, `grep` — when agent sees `use tokio::sync::mpsc`, it can fetch tokio docs for better understanding.
