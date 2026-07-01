# Configuration

Runie is configured via `~/.runie/config.toml`. Config hot-reloads while you type.

## Canonical Config Example

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

## Provider Configuration

Each provider requires an API key. Configure multiple providers:

```toml
[model_providers.anthropic]
type = "anthropic"
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

## Model Selection

The `scoped` list controls which models appear in the model selector:

```toml
[models]
# Default model (overridden by --model flag)
scoped = [
  "anthropic/claude-sonnet-4-6",
  "gpt-4o",
  "deepseek/deepseek-chat",
]
```

## Permissions

Control which tools require approval. Rules are checked top-to-bottom; the first match wins.

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

## Environment & Secrets

Provider API keys can be set via environment variables or stored in the OS keyring:

```toml
[model_providers.anthropic]
type = "anthropic"
# Key from ANTHROPIC_API_KEY env var, or OS keyring
api_key = "$ANTHROPIC_API_KEY"
```

Supported environment variables per provider:
- `ANTHROPIC_API_KEY` — Anthropic
- `OPENAI_API_KEY` — OpenAI
- `DEEPSEEK_API_KEY` — DeepSeek
- `OPENAI_API_KEY` — any OpenAI-compatible provider

Runie looks up the keyring entry keyed by `"runie"` when the value is empty or starts with `$`.

## Truncation Policy

Control how Runie truncates conversation history:

```toml
[truncation]
# Maximum lines to keep in context
max_lines = 2000
# Maximum bytes per message
max_bytes = 51200
```

## Telemetry

Anonymous usage telemetry (disabled by default):

```toml
[telemetry]
enabled = false
```

## Keybindings

Keybindings can be customized in config:

```toml
[keybindings]
# Custom keybinding examples (using crossterm syntax)
# Ctrl+P = open command palette
# Ctrl+G = toggle session tree
```

## Related

- [Architecture](Architecture.md) — runtime model and event system
- [UI/UX](UI_UX.md) — interaction patterns
