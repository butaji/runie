# Custom Prompt Templates

**Status**: todo
**Milestone**: R3
**Category**: Configuration

## Description

User-defined system prompt overrides. Users can customize the system prompt via config or per-session.

## Architecture

```rust
pub struct PromptTemplate {
    pub name: String,
    pub content: String,
    pub source: PromptSource,
}

pub enum PromptSource {
    BuiltIn,
    UserFile(PathBuf),
    ProjectFile(PathBuf),
}

// In config.toml
// [prompts]
// default = "You are a helpful coding assistant."
// custom = "path/to/prompt.md"
```

## Acceptance Criteria

- [ ] `prompts.default` in config.toml sets system prompt
- [ ] `prompts.custom` loads from file path
- [ ] `/prompt <name>` switches prompt template
- [ ] Prompt shown in session info
- [ ] Prompts reload with `/reload`

## Tests

### Layer 1
- [ ] `prompt_loaded_from_config` — config field parsed
- [ ] `prompt_loaded_from_file` — file content read
- [ ] `prompt_switch_updates` — /prompt changes system message
