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
provider_type = "anthropic"
api_key = "sk-ant-..."

[model_providers.openai]
provider_type = "openai"
base_url = "https://api.openai.com/v1"
api_key = "sk-..."

[model_providers.deepseek]
provider_type = "openai-compatible"
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
