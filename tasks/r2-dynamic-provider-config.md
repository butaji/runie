# Dynamic Provider Config Resolution

**Status**: done
**Milestone**: R2
**Category**: Configuration

## Description

Resolve provider configuration from multiple sources: env vars, files, CLI flags. pi supports `OPENAI_API_KEY` env var, `.env` files, and per-provider config sections.

## Architecture

```rust
pub struct ProviderConfigResolver {
    env: HashMap<String, String>,
    dotenv: HashMap<String, String>,
    config_file: HashMap<String, ProviderConfig>,
}

impl ProviderConfigResolver {
    pub fn resolve_api_key(&self, provider: &str) -> Option<String> {
        // Priority: env var > .env file > config.toml
        let env_key = format!("{}_API_KEY", provider.to_uppercase());
        self.env.get(&env_key)
            .or_else(|| self.dotenv.get(&env_key))
            .or_else(|| self.config_file.get(provider)?.api_key.as_ref())
            .cloned()
    }

    pub fn resolve_base_url(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_BASE_URL", provider.to_uppercase());
        self.env.get(&env_key)
            .or_else(|| self.dotenv.get(&env_key))
            .or_else(|| self.config_file.get(provider)?.base_url.as_ref())
            .cloned()
    }
}
```

## Acceptance Criteria

- [ ] Reads API key from `PROVIDER_API_KEY` env var
- [ ] Reads from `.env` file in cwd
- [ ] Falls back to config.toml `model_providers` section
- [ ] Resolves base_url from env/config
- [ ] Works for all 35 providers

## Tests

### Layer 1
- [ ] `resolve_env_takes_priority` — env var wins over config
- [ ] `resolve_dotenv_fallback` — .env used when env not set
- [ ] `resolve_config_fallback` — config used when neither env nor .env
