//! Subagent Types — declarative built-in agent profiles.
//!
//! Subagent types are defined as markdown files with YAML frontmatter in
//! `resources/agents/`.  Each file defines the prompt, model, permission mode,
//! and other settings.  User overrides live under `~/.runie/agents/`.
//!
//! # File format
//!
//! ```markdown
//! ---
//! name: explore
//! description: Fast codebase exploration for patterns and architecture.
//! prompt_mode: full
//! model: inherit
//! permission_mode: default
//! agents_md: true
//! ---
//!
//! You are an expert explorer. ...
//! ```
//!
//! # Loading
//!
//! - `SubagentRegistry::from_builtins()` loads all built-in types embedded at
//!   compile time via `include_str!`.
//! - `SubagentRegistry::load_user_overrides()` extends the registry with types
//!   from `~/.runie/agents/`.  User types override built-ins of the same name.

use camino::Utf8PathBuf;
use std::collections::HashMap;

use tinytemplate::TinyTemplate;

use crate::resource_loader::{extract_body, extract_frontmatter};

/// Prompt mode for a subagent — controls how much context is included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, strum::EnumString)]
pub enum PromptMode {
    /// Full context: all session history, AGENTS.md, all skills.
    #[default]
    Full,
    /// Compact context: recent messages, no extra preamble.
    Compact,
}

// Re-export canonical PermissionMode from permissions.
pub use crate::permissions::PermissionMode;

/// A loaded subagent type definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentType {
    /// Unique type identifier (e.g. "explore", "plan").
    pub name: String,
    /// Human-readable description of when to use this type.
    pub description: String,
    /// How much context to include.
    pub prompt_mode: PromptMode,
    /// Model spec: concrete id, "inherit", or "fast" trait.
    pub model: String,
    /// Permission mode for this subagent.
    pub permission_mode: PermissionMode,
    /// Whether to inject project AGENTS.md into context.
    pub agents_md: bool,
    /// The prompt template body (markdown).
    pub body: String,
}

impl SubagentType {
    /// Interpolate `{{variable}}` placeholders in the body using tinytemplate.
    /// Variables not in `vars` are rendered as empty strings.
    pub fn interpolate(&self, vars: &HashMap<&str, &str>) -> String {
        // Convert {{var}} syntax to tinytemplate's {var} syntax.
        let template_body = convert_braces(&self.body);
        // Extract all variable names from the template to pre-populate context.
        let var_names = extract_var_names(&template_body);
        let mut context: HashMap<&str, &str> = HashMap::new();
        for (k, v) in vars {
            context.insert(*k, *v);
        }
        for name in &var_names {
            context.entry(name.as_str()).or_insert("");
        }
        let mut tt = TinyTemplate::new();
        match tt.add_template("body", &template_body) {
            Ok(()) => tt.render("body", &context).unwrap_or_else(|e| {
                tracing::warn!("Render error: {:?}", e);
                self.body.clone()
            }),
            Err(e) => {
                tracing::warn!("Add template error: {:?}", e);
                self.body.clone()
            }
        }
    }
}

/// Extract all variable names from a tinytemplate template.
fn extract_var_names(template: &str) -> Vec<String> {
    let mut names = Vec::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] != '{' {
            // Found {name}, extract the name.
            i += 1; // consume {
            let start = i;
            while i < chars.len() && chars[i] != '}' {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                names.push(name);
            }
            if i < chars.len() {
                i += 1; // consume }
            }
        } else {
            i += 1;
        }
    }
    names
}

/// Convert `{{var}}` template syntax to tinytemplate's `{var}` syntax.
fn convert_braces(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for {{ (start of variable placeholder)
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
            result.push('{');
            i += 2; // skip both {
        }
        // Check for }} (end of variable placeholder)
        else if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            result.push('}');
            i += 2; // skip both }
        }
        // Single brace or non-brace character
        else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// Registry of all available subagent types.
#[derive(Debug, Clone, Default)]
pub struct SubagentRegistry {
    types: HashMap<String, SubagentType>,
}

impl SubagentRegistry {
    /// Create a registry with all built-in (embedded) subagent types.
    pub fn from_builtins() -> Self {
        let mut types = HashMap::new();
        for (name, st) in embedded_types() {
            types.insert(name.to_owned(), st);
        }
        Self { types }
    }

    /// Load user overrides from `~/.runie/agents/`.  Files with the same
    /// `name` as a built-in type replace the built-in.
    pub fn load_user_overrides(&mut self) {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let user_dir = home.join(".runie").join("agents");
        for entry in std::fs::read_dir(&user_dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            if let Ok(path_utf8) = Utf8PathBuf::from_path_buf(path) {
                if let Some(st) = parse_subagent_file(&path_utf8) {
                    self.types.insert(st.name.clone(), st);
                }
            }
        }
    }

    /// Get a subagent type by name, or `None` if not found.
    pub fn get(&self, name: &str) -> Option<&SubagentType> {
        self.types.get(name)
    }

    /// Iterate over all registered types.
    pub fn iter(&self) -> impl Iterator<Item = &SubagentType> {
        self.types.values()
    }

    /// Number of registered types.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

/// Parse a subagent markdown file from disk, returning `None` on error.
pub fn parse_subagent_file(path: &Utf8PathBuf) -> Option<SubagentType> {
    let content = std::fs::read_to_string(path.as_std_path()).ok()?;
    let stem = path.file_stem().map(String::from)?;
    parse_subagent_content(&stem, &content)
}

/// Extract a field from the frontmatter map, or return the default.
fn fm_str(fm: &HashMap<String, String>, key: &str) -> String {
    fm.get(key).cloned().unwrap_or_default()
}

/// Parse subagent content (YAML frontmatter + markdown body).
/// `name_hint` is used when the frontmatter has no `name` field.
/// Returns `None` only on I/O errors (not on missing frontmatter).
fn parse_subagent_content(name_hint: &str, content: &str) -> Option<SubagentType> {
    let fm = extract_frontmatter(content);
    let name = fm_str(&fm, "name");
    let name = if name.is_empty() {
        name_hint.to_owned()
    } else {
        name
    };
    let prompt_mode = match fm_str(&fm, "prompt_mode").as_str() {
        "compact" => PromptMode::Compact,
        _ => PromptMode::Full,
    };
    let permission_mode =
        crate::permissions::parse_permission_mode(&fm_str(&fm, "permission_mode"));
    let agents_md = fm_str(&fm, "agents_md").parse::<bool>().unwrap_or(false);
    let model = fm_str(&fm, "model");
    let model = if model.is_empty() {
        "inherit".to_owned()
    } else {
        model
    };
    Some(SubagentType {
        name,
        description: fm_str(&fm, "description"),
        prompt_mode,
        model,
        permission_mode,
        agents_md,
        body: extract_body(content),
    })
}

// ── Embedded types ────────────────────────────────────────────────────────────
// These are embedded at compile time via `include_str!`.  Their SHA-256
// checksums are validated at build time in `build.rs`.

/// Hardcoded embedded subagent types — parsed at module init time.
fn embedded_types() -> Vec<(&'static str, SubagentType)> {
    vec![
        (
            "explore",
            parse_subagent_content("explore", include_str!("../../resources/agents/explore.md"))
                .expect("embedded explore must parse"),
        ),
        (
            "plan",
            parse_subagent_content("plan", include_str!("../../resources/agents/plan.md"))
                .expect("embedded plan must parse"),
        ),
        (
            "verify",
            parse_subagent_content("verify", include_str!("../../resources/agents/verify.md"))
                .expect("embedded verify must parse"),
        ),
        (
            "check-work",
            parse_subagent_content(
                "check-work",
                include_str!("../../resources/agents/check-work.md"),
            )
            .expect("embedded check-work must parse"),
        ),
    ]
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Layer 1 — State/Logic

    #[test]
    fn registry_loads_all_builtin_types() {
        let reg = SubagentRegistry::from_builtins();
        assert_eq!(reg.len(), 4);
        assert!(reg.get("explore").is_some());
        assert!(reg.get("plan").is_some());
        assert!(reg.get("verify").is_some());
        assert!(reg.get("check-work").is_some());
    }

    #[test]
    fn explore_type_has_correct_fields() {
        let reg = SubagentRegistry::from_builtins();
        let explore = reg.get("explore").unwrap();
        assert_eq!(explore.name, "explore");
        assert!(!explore.description.is_empty());
        assert_eq!(explore.prompt_mode, PromptMode::Full);
        assert_eq!(explore.model, "inherit");
        assert_eq!(explore.permission_mode, PermissionMode::Default);
        assert!(explore.agents_md);
        assert!(explore.body.contains("expert explorer"));
    }

    #[test]
    fn plan_type_uses_plan_permission_mode() {
        let reg = SubagentRegistry::from_builtins();
        let plan = reg.get("plan").unwrap();
        assert_eq!(plan.permission_mode, PermissionMode::Plan);
        assert_eq!(plan.prompt_mode, PromptMode::Full);
        assert!(plan.agents_md);
    }

    #[test]
    fn verify_type_uses_compact_mode_and_auto_permission() {
        let reg = SubagentRegistry::from_builtins();
        let verify = reg.get("verify").unwrap();
        assert_eq!(verify.prompt_mode, PromptMode::Compact);
        assert_eq!(verify.permission_mode, PermissionMode::Auto);
    }

    #[test]
    fn interpolate_replaces_variables() {
        let reg = SubagentRegistry::from_builtins();
        let explore = reg.get("explore").unwrap();
        let mut vars = HashMap::new();
        vars.insert("task", "find all TODO comments");
        let interpolated = explore.interpolate(&vars);
        assert!(interpolated.contains("find all TODO comments"));
    }

    #[test]
    fn interpolate_renders_single_braces_not_in_vars() {
        let st = SubagentType {
            name: "test".into(),
            description: "".into(),
            prompt_mode: PromptMode::Full,
            model: "inherit".into(),
            permission_mode: PermissionMode::Default,
            agents_md: false,
            body: "task: {{task}}, single brace: {literal}".into(),
        };
        let vars: HashMap<&str, &str> = [("task", "do it")].into();
        let interpolated = st.interpolate(&vars);
        assert!(interpolated.contains("task: do it"));
        // Single braces get rendered as empty by TinyTemplate since they're not in vars
        assert!(interpolated.contains("task: do it, single brace: "));
    }

    #[test]
    fn user_override_replaces_builtin() {
        let mut reg = SubagentRegistry::from_builtins();
        let original_body = reg.get("explore").unwrap().body.clone();
        reg.types.insert(
            "explore".to_owned(),
            SubagentType {
                name: "explore".to_owned(),
                description: "custom".to_owned(),
                prompt_mode: PromptMode::Compact,
                model: "fast".to_owned(),
                permission_mode: PermissionMode::Auto,
                agents_md: false,
                body: "custom body".to_owned(),
            },
        );
        assert_eq!(reg.get("explore").unwrap().body, "custom body");
        // Unaffected types still work.
        assert!(reg.get("plan").is_some());
        drop(original_body); // suppress unused warning
    }

    #[test]
    fn parse_content_with_full_frontmatter() {
        let content = r#"---
name: test-type
description: A test type.
prompt_mode: compact
model: fast
permission_mode: auto
agents_md: false
---

This is the body.
"#;
        let st = parse_subagent_content("test-type", content).unwrap();
        assert_eq!(st.name, "test-type");
        assert_eq!(st.description, "A test type.");
        assert_eq!(st.prompt_mode, PromptMode::Compact);
        assert_eq!(st.model, "fast");
        assert_eq!(st.permission_mode, PermissionMode::Auto);
        assert!(!st.agents_md);
        assert_eq!(st.body, "This is the body.");
    }

    #[test]
    fn parse_content_minimal_frontmatter_uses_defaults() {
        let content = r#"---
name: minimal
---

Minimal body.
"#;
        let st = parse_subagent_content("minimal", content).unwrap();
        assert_eq!(st.name, "minimal");
        assert_eq!(st.description, "");
        assert_eq!(st.prompt_mode, PromptMode::Full);
        assert_eq!(st.model, "inherit");
        assert_eq!(st.permission_mode, PermissionMode::Default);
        assert!(!st.agents_md);
        assert_eq!(st.body, "Minimal body.");
    }

    #[test]
    fn parse_content_no_frontmatter_uses_hint_and_content() {
        let content = "No frontmatter, just body.";
        let st = parse_subagent_content("no-fm", content).unwrap();
        assert_eq!(st.name, "no-fm");
        assert_eq!(st.body, "No frontmatter, just body.");
    }

    #[test]
    fn parse_content_multi_paragraph_body() {
        let content = r#"---
name: multi
---

First paragraph.

Second paragraph.

Third paragraph.
"#;
        let st = parse_subagent_content("multi", content).unwrap();
        assert_eq!(
            st.body,
            "First paragraph.\n\nSecond paragraph.\n\nThird paragraph."
        );
    }

    #[test]
    fn permission_mode_parse_all_variants() {
        use crate::permissions::parse_permission_mode;
        assert_eq!(
            parse_permission_mode("acceptEdits"),
            PermissionMode::AcceptEdits
        );
        assert_eq!(parse_permission_mode("auto"), PermissionMode::Auto);
        assert_eq!(parse_permission_mode("dontAsk"), PermissionMode::DontAsk);
        assert_eq!(
            parse_permission_mode("bypassPermissions"),
            PermissionMode::BypassPermissions
        );
        assert_eq!(parse_permission_mode("plan"), PermissionMode::Plan);
        assert_eq!(parse_permission_mode("unknown"), PermissionMode::Default);
    }

    #[test]
    fn convert_braces_converts_template_syntax() {
        assert_eq!(convert_braces("{{var}}"), "{var}");
        assert_eq!(convert_braces("{{task}} is done"), "{task} is done");
        assert_eq!(
            convert_braces("task: {{task}}, user: {{user}}"),
            "task: {task}, user: {user}"
        );
    }

    #[test]
    fn convert_braces_preserves_single_braces() {
        assert_eq!(convert_braces("{single}"), "{single}");
        assert_eq!(
            convert_braces("{{double}} and {single}"),
            "{double} and {single}"
        );
        assert_eq!(
            convert_braces("text with {brace} inside"),
            "text with {brace} inside"
        );
    }

    #[test]
    fn convert_braces_handles_empty_and_edge_cases() {
        assert_eq!(convert_braces(""), "");
        assert_eq!(convert_braces("no braces"), "no braces");
        // Triple opening, three closing: {{ + { + }} + } → {{}}
        assert_eq!(convert_braces("{{{}}}"), "{{}}");
        // Four opening, two closing: {{ + {{ + }} → {{{
        assert_eq!(convert_braces("{{{{}}"), "{{}");
    }

    #[test]
    fn interpolate_renders_known_variables() {
        let st = SubagentType {
            name: "test".into(),
            description: "".into(),
            prompt_mode: PromptMode::Full,
            model: "inherit".into(),
            permission_mode: PermissionMode::Default,
            agents_md: false,
            body: "Task: {{task}}, User: {{user}}".into(),
        };
        let vars: HashMap<&str, &str> = [("task", "fix bug"), ("user", "alice")].into();
        let result = st.interpolate(&vars);
        assert!(result.contains("Task: fix bug"));
        assert!(result.contains("User: alice"));
    }

    #[test]
    fn interpolate_renders_unknown_as_empty() {
        let st = SubagentType {
            name: "test".into(),
            description: "".into(),
            prompt_mode: PromptMode::Full,
            model: "inherit".into(),
            permission_mode: PermissionMode::Default,
            agents_md: false,
            body: "task: {{task}}, unknown: {{missing}}".into(),
        };
        let vars: HashMap<&str, &str> = [("task", "do it")].into();
        let result = st.interpolate(&vars);
        // Known variable is rendered
        assert!(result.contains("task: do it"));
        // Unknown variable {{missing}} renders as empty (TinyTemplate behavior)
        assert!(result.contains("task: do it, unknown: "));
    }
}
