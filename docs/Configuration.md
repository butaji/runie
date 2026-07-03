# Configuration

Runie reads `~/.runie/config.toml`. Config hot-reloads while you type.

## Canonical example

```toml
provider = "anthropic"
model = "claude-sonnet-4-6"

[models]
scoped = ["claude-sonnet-4-6", "gpt-4o", "deepseek-chat"]

[truncation]
max_lines = 2000
max_bytes = 51200

[telemetry]
enabled = false
```

## Providers

Each provider needs an API key:

```toml
[model_providers.anthropic]
type = "anthropic"
base_url = "https://api.anthropic.com"
api_key = "sk-ant-..."

[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com/v1"
api_key = "sk-..."

[model_providers.deepseek]
type = "openai-compatible"
base_url = "https://api.deepseek.com/v1"
api_key = "sk-..."
```

## Model selection

The `scoped` list controls which models appear in the selector:

```toml
[models]
scoped = [
  "anthropic/claude-sonnet-4-6",
  "gpt-4o",
  "deepseek/deepseek-chat",
]
```

## Permissions

Rules are checked top-to-bottom; the first match wins.

```toml
[[permissions]]
action = "allow"
tool = "read_file"

[[permissions]]
action = "ask"
tool = "bash"
pattern = "git push"

[[permissions]]
action = "deny"
tool = "rm"
pattern = "rm -rf /"
```

## Environment & secrets

API keys can be set via environment variables or the OS keyring. Use `$VAR` in the config to pull from the environment:

```toml
[model_providers.anthropic]
api_key = "$ANTHROPIC_API_KEY"
```

Supported env vars: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `DEEPSEEK_API_KEY`. Any OpenAI-compatible provider also reads `OPENAI_API_KEY`.

Runie looks up the keyring entry keyed by `"runie"` when the value is empty or starts with `$`.

## Truncation

```toml
[truncation]
max_lines = 2000
max_bytes = 51200
```

## Telemetry

```toml
[telemetry]
enabled = false
```

## Related

- [Architecture](Architecture.md) — runtime model and event system
- [UI/UX](UI_UX.md) — interaction patterns
