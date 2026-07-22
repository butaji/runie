//! Custom prompt templates — user-defined system prompt overrides.

/// Source of a prompt template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptSource {
    BuiltIn,
    /// User-configured prompt file path (stored as string for UTF-8 safety).
    UserFile(String),
    /// Project-configured prompt file path (stored as string for UTF-8 safety).
    ProjectFile(String),
}

/// A loaded prompt template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptTemplate {
    pub name: String,
    pub content: String,
    pub source: PromptSource,
}

impl PromptTemplate {
    /// Build a one-line summary for listing.
    pub fn summary(&self) -> String {
        let src = match self.source {
            PromptSource::BuiltIn => "built-in",
            PromptSource::UserFile(_) => "user",
            PromptSource::ProjectFile(_) => "project",
        };
        format!("{} ({})", self.name, src)
    }
}

/// Default system prompt used when no custom prompt is configured.
/// Loaded from resources so editing does not require recompile.
pub const DEFAULT_PROMPT: &str = include_str!("../resources/prompts/default.txt");

/// Default tool list advertised to the model.
/// Loaded from resources so editing does not require recompile.
pub const DEFAULT_TOOLS: &str = include_str!("../resources/tools/default.txt");

/// Load prompt templates from config values.
pub fn load_prompts(default: Option<&str>, custom_path: Option<&str>) -> Vec<PromptTemplate> {
    let mut prompts = Vec::new();

    prompts.push(PromptTemplate {
        name: "default".into(),
        content: default.unwrap_or(DEFAULT_PROMPT).into(),
        source: PromptSource::BuiltIn,
    });

    if let Some(path) = custom_path {
        if let Ok(content) = tokio::task::block_in_place(move || std::fs::read_to_string(path)) {
            prompts.push(PromptTemplate {
                name: "custom".into(),
                content: content.trim().into(),
                source: PromptSource::UserFile(path.to_owned()),
            });
        }
    }

    prompts
}

/// Build the system prompt string for the agent.
/// If a custom prompt is active, it replaces the base personality text.
/// Tool instructions are appended only when `tools_list` is non-empty, and the
/// thinking suffix is always appended when provided.
pub fn build_system_prompt(base_prompt: &str, tools_list: &str, read_only: bool, thinking_suffix: &str) -> String {
    let mut system = base_prompt.to_owned();
    if !tools_list.is_empty() {
        system.push_str(&format!(
            " Use structured JSON format: {{\"name\": \"tool_name\", \"arguments\": {{...}}}}. \
                Available tools: {}. \
                When a task can be satisfied with a tool, prefer the tool over answering from memory.",
            tools_list
        ));
        if !read_only {
            system.push_str(" \
                Use edit_file for safe changes: {\"name\": \"edit_file\", \"arguments\": {\"path\": \"...\", \"search\": \"...\", \"replace\": \"...\"}}.");
        }
        system.push_str(" \
            Use grep to search file contents: {\"name\": \"grep\", \"arguments\": {\"pattern\": \"...\", \"path\": \"...\"}}. \
            Use find to list files by pattern: {\"name\": \"find\", \"arguments\": {\"pattern\": \"...\", \"path\": \"...\"}}.");
    }
    if !thinking_suffix.is_empty() {
        system.push_str(thinking_suffix);
    }
    system
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn prompt_loaded_from_config() {
        let prompts = load_prompts(Some("Be concise."), None);
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, "default");
        assert_eq!(prompts[0].content, "Be concise.");
        assert_eq!(prompts[0].source, PromptSource::BuiltIn);
    }

    #[test]
    fn prompt_loaded_from_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("prompt.md");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"You are a Rust expert.").unwrap();

        let prompts = load_prompts(None, Some(path.to_str().unwrap()));
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[1].name, "custom");
        assert_eq!(prompts[1].content, "You are a Rust expert.");
        assert!(matches!(prompts[1].source, PromptSource::UserFile(_)));
    }

    #[test]
    fn build_system_prompt_includes_tools() {
        let prompt = build_system_prompt("Be helpful.", "read_file, bash", false, "");
        assert!(prompt.contains("Be helpful."));
        assert!(prompt.contains("read_file"));
        assert!(prompt.contains("bash"));
    }

    #[test]
    fn build_system_prompt_appends_thinking() {
        let prompt = build_system_prompt("Hi.", "tools", false, " Think deeply.");
        assert!(prompt.contains("Hi."));
        assert!(prompt.contains("Think deeply."));
    }

    #[test]
    fn build_system_prompt_prefers_tools() {
        let prompt = build_system_prompt("Hi.", "read_file, bash", false, "");
        assert!(prompt.contains("prefer the tool over answering from memory"));
    }

    #[test]
    fn build_system_prompt_omits_tools_when_list_empty() {
        let prompt = build_system_prompt("Be helpful.", "", false, "");
        assert!(prompt.contains("Be helpful."));
        assert!(!prompt.contains("Available tools"));
        assert!(!prompt.contains("Use structured JSON format"));
        assert!(!prompt.contains("edit_file"));
        assert!(!prompt.contains("grep"));
    }

    #[test]
    fn build_system_prompt_appends_thinking_without_tools() {
        let prompt = build_system_prompt("Hi.", "", false, " Think deeply.");
        assert!(prompt.contains("Hi."));
        assert!(prompt.contains("Think deeply."));
        assert!(!prompt.contains("Available tools"));
    }
}
