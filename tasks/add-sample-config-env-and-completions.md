# Add sample config, env template, and completions

## Status

`done`

## Context

Only `config.schema.json` existed; new users lacked a working TOML reference, env var list, and shell completions.

## Goal

Add `docs/config.example.toml`, `.env.example`, and a `clap_complete`-based completion generator.

## Acceptance Criteria

- [x] Create sample config with provider/keyring/MCP blocks — `docs/config.example.toml`
- [x] Create `.env.example` — `.env.example` with all provider env vars
- [x] Add completion generator — `crates/runie-cli/src/completion.rs` with clap_complete

## Implementation

### Sample Config
`docs/config.example.toml` contains a comprehensive reference including:
- Provider configuration (anthropic, openai, deepseek, grok, minimax, ollama)
- Model selection with scoped list
- Truncation settings
- Permission rules
- UI settings (vim_mode, thinking_level)
- Telemetry
- Hooks, keybindings, prompts
- Theme
- MCP server configurations

### Env Template
`.env.example` contains all provider API key environment variables:
- ANTHROPIC_API_KEY
- OPENAI_API_KEY
- DEEPSEEK_API_KEY
- XAI_API_KEY
- MINIMAX_API_KEY
- OLLAMA_BASE_URL
- LMSTUDIO_BASE_URL

### Completion Generator
`crates/runie-cli/src/completion.rs` provides shell completion generation:
- Feature-gated with `completions` feature
- Supports bash, zsh, fish, powershell, elvish
- Usage: `runie completion bash > /etc/bash_completion.d/runie`

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** CI diff-check keeps samples in sync with schema.
- **Live tmux testing session (required):** N/A.

## Completion Validation

- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes
