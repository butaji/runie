# OAuth Authentication (/login, /logout)

**Status**: done
**Milestone**: R3
**Category**: Configuration

## Description

Per-provider OAuth login/logout. Supports device auth flow for providers that require it.

## Architecture

```rust
pub struct AuthStorage {
    tokens: HashMap<String, AuthToken>,
    path: PathBuf,
}

pub struct AuthToken {
    pub provider: String,
    pub token: String,
    pub expires_at: Option<f64>,
}

fn cmd_login(args: &str) -> Option<Event> {
    Some(Event::LoginProvider { provider: args.trim().to_string() })
}

fn cmd_logout(args: &str) -> Option<Event> {
    Some(Event::LogoutProvider { provider: args.trim().to_string() })
}
```

### OAuth Device Flow

```
1. User types /login openai
2. If provider supports device flow:
   a. Request device code from provider
   b. Show user code + verification URL
   c. Poll for token
   d. Store token in ~/.runie/auth.json
3. If API key provider: prompt for key
```

## Acceptance Criteria

- [x] `/login <provider>` initiates auth flow
- [x] `/logout <provider>` removes stored token
- [ ] Device flow for supported providers (GitHub Copilot, etc.) — future work
- [x] API key prompt for key-based providers (`/login <provider> <token>`)
- [x] Tokens stored encrypted in `~/.runie/auth.json`
- [x] Token refresh before expiry
- [x] Login status shown in status bar

## Tests

### Layer 1
- [x] `auth_storage_save_load` — JSON roundtrip
- [x] `token_refresh_needed` — detects expired token
