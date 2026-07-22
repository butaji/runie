#![allow(clippy::too_many_lines)]

//! Skill discovery and YAML frontmatter parsing (from Grok Build)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Skill subdirectories to search
const SKILL_SUBDIRS: &[&str] = &["skills"];

/// Maximum depth for skill walk
const MAX_SKILL_WALK_DEPTH: usize = 5;

/// Skill discovery path with scope
#[derive(Debug, Clone)]
pub struct SkillLocation {
    /// Path to the skill
    pub path: PathBuf,
    /// Scope of the skill
    pub scope: SkillScope,
}

impl SkillLocation {
    /// Get all discovery paths for a given working directory
    pub fn discovery_paths(cwd: &Path) -> Vec<Self> {
        let mut paths = Vec::new();

        // Local: CWD/.runie/skills/
        if let Ok(cwd) = std::env::current_dir() {
            paths.push(Self {
                path: cwd.join(".runie/skills"),
                scope: SkillScope::Local,
            });
        }

        // Repo: find .git root
        if let Some(repo_root) = find_git_root(cwd) {
            paths.push(Self {
                path: repo_root.join(".runie/skills"),
                scope: SkillScope::Repo,
            });
        }

        // User: ~/.runie/skills/
        if let Some(home) = dirs::home_dir() {
            paths.push(Self {
                path: home.join(".runie/skills"),
                scope: SkillScope::User,
            });
        }

        // Claude compat: ~/.claude/skills/
        if let Some(home) = dirs::home_dir() {
            paths.push(Self {
                path: home.join(".claude/skills"),
                scope: SkillScope::ClaudeCompat,
            });
        }

        paths.sort_by_key(|p| p.scope);
        paths
    }

    /// Check if this path exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Get the path as a string
    pub fn as_str(&self) -> Option<&str> {
        self.path.to_str()
    }
}

/// Skill scope (priority order)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillScope {
    /// Local to current directory
    Local,
    /// Repository-level
    Repo,
    /// User-level (~/.runie/)
    User,
    /// Claude compatibility (~/.claude/)
    ClaudeCompat,
}

impl SkillScope {
    /// Get the directory name for this scope
    pub fn dir_name(&self) -> &'static str {
        match self {
            SkillScope::Local | SkillScope::Repo => ".runie/skills",
            SkillScope::User => ".runie/skills",
            SkillScope::ClaudeCompat => ".claude/skills",
        }
    }

    /// Check if this scope should be auto-loaded
    pub fn is_auto_load(&self) -> bool {
        matches!(self, SkillScope::Local | SkillScope::Repo)
    }
}

/// Find all skill paths recursively
pub fn find_skill_paths(dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    walk_for_skill_md(dir, &mut paths, 0);
    paths
}

fn walk_for_skill_md(dir: &Path, paths: &mut Vec<PathBuf>, depth: usize) {
    if depth >= MAX_SKILL_WALK_DEPTH {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            // Check for SKILL.md in directory
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                paths.push(skill_md);
            }

            // Recurse into skill subdirectories
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if SKILL_SUBDIRS.contains(&name) {
                    walk_for_skill_md(&path, paths, depth + 1);
                }
            }

            // Also recurse one level into other directories (for flat skill files)
            if depth < 2 {
                walk_for_skill_md(&path, paths, depth + 1);
            }
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            // Check if filename looks like a skill
            if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                if name.to_lowercase().contains("skill") || is_skill_file(&path) {
                    paths.push(path);
                }
            }
        }
    }
}

fn is_skill_file(path: &Path) -> bool {
    // Check for YAML frontmatter with skill indicators
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Some((frontmatter, _)) = parse_frontmatter(&content) {
            return frontmatter.contains_key("name")
                || frontmatter.contains_key("description")
                || frontmatter.contains_key("trigger");
        }
    }
    false
}

/// Parse YAML frontmatter with type coercion
pub fn parse_frontmatter(content: &str) -> Option<(HashMap<String, String>, &str)> {
    if !content.trim().starts_with("---") {
        return None;
    }

    let after_first = content.trim_start_matches("---");
    let end = after_first.find("---")?;

    let yaml_str = after_first[..end].trim();
    let body = &after_first[end + 3..];

    let yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str).ok()?;
    let mut map = HashMap::new();

    if let Some(map_yaml) = yaml.as_mapping() {
        for (k, v) in map_yaml {
            let key = k.as_str()?.to_string();
            if let Some(value) = coerce_to_string(Some(v)) {
                map.insert(key, value);
            }
        }
    }

    Some((map, body.trim()))
}

fn coerce_to_string(value: Option<&serde_yaml::Value>) -> Option<String> {
    match value? {
        serde_yaml::Value::String(s) => Some(s.trim().to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Null => Some(String::new()),
        _ => None,
    }
}

/// Parse a boolean value from YAML
pub fn parse_boolean_frontmatter(value: Option<&serde_yaml::Value>) -> bool {
    match value {
        Some(serde_yaml::Value::Bool(true)) => true,
        Some(serde_yaml::Value::String(s)) => s.trim().eq_ignore_ascii_case("true"),
        Some(serde_yaml::Value::Number(n)) => n.as_i64().map(|v| v != 0).unwrap_or(false),
        _ => false,
    }
}

/// Parse an array from YAML
pub fn parse_array_frontmatter(value: Option<&serde_yaml::Value>) -> Vec<String> {
    match value {
        Some(serde_yaml::Value::Sequence(arr)) => {
            arr.iter()
                .filter_map(|v| coerce_to_string(Some(v)))
                .collect()
        }
        Some(serde_yaml::Value::String(s)) => {
            s.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Find git root directory
fn find_git_root(cwd: &Path) -> Option<PathBuf> {
    let mut dir = cwd.to_path_buf();

    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }

        if !dir.pop() {
            return None;
        }
    }
}

/// A discovered skill
#[derive(Debug, Clone)]
pub struct Skill {
    /// Skill name
    pub name: String,
    /// Skill description
    pub description: Option<String>,
    /// Path to the skill file
    pub path: PathBuf,
    /// Skill scope
    pub scope: SkillScope,
    /// Triggers for this skill
    pub triggers: Vec<String>,
    /// Whether this skill is enabled
    pub enabled: bool,
    /// Raw frontmatter
    pub frontmatter: HashMap<String, String>,
    /// Skill content (after frontmatter)
    pub content: String,
}

impl Skill {
    /// Load a skill from a path
    pub fn from_path(path: &Path, scope: SkillScope) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let (frontmatter, body) = parse_frontmatter(&content)
            .unwrap_or_else(|| (HashMap::new(), content.as_str()));

        let name = frontmatter
            .get("name")
            .cloned()
            .or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Unnamed".to_string());

        let description = frontmatter.get("description").cloned();
        let triggers = frontmatter
            .get("trigger")
            .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        let enabled = frontmatter
            .get("enabled")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        Ok(Self {
            name,
            description,
            path: path.to_path_buf(),
            scope,
            triggers,
            enabled,
            frontmatter,
            content: body.to_string(),
        })
    }

    /// Check if a prompt matches this skill's triggers
    pub fn matches(&self, prompt: &str) -> bool {
        if self.triggers.is_empty() {
            return false;
        }

        let prompt_lower = prompt.to_lowercase();
        self.triggers
            .iter()
            .any(|trigger| prompt_lower.contains(&trigger.to_lowercase()))
    }

    /// Get the content as a prompt to prepend
    pub fn as_system_prompt(&self) -> String {
        let mut prompt = String::new();

        if let Some(ref desc) = self.description {
            prompt.push_str(&format!("# {}\n\n{}\n\n", self.name, desc));
        } else {
            prompt.push_str(&format!("# {}\n\n", self.name));
        }

        prompt.push_str(self.content.trim());
        prompt.push('\n');

        prompt
    }
}

/// Skill registry for managing discovered skills
#[derive(Default)]
pub struct SkillRegistry {
    skills: Vec<Skill>,
    by_scope: HashMap<SkillScope, Vec<usize>>,
    by_trigger: HashMap<String, Vec<usize>>,
}

impl SkillRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Discover skills from paths
    pub fn discover(cwd: &Path) -> anyhow::Result<Self> {
        let mut registry = Self::new();
        let paths = SkillLocation::discovery_paths(cwd);

        for location in paths {
            if !location.exists() {
                continue;
            }

            let skill_paths = find_skill_paths(&location.path);
            for path in skill_paths {
                match Skill::from_path(&path, location.scope) {
                    Ok(skill) => {
                        registry.add(skill);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load skill {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(registry)
    }

    /// Add a skill to the registry
    pub fn add(&mut self, skill: Skill) {
        let idx = self.skills.len();
        self.skills.push(skill.clone());

        // Index by scope
        self.by_scope
            .entry(skill.scope)
            .or_default()
            .push(idx);

        // Index by trigger
        for trigger in &skill.triggers {
            self.by_trigger
                .entry(trigger.to_lowercase())
                .or_default()
                .push(idx);
        }
    }

    /// Get all skills
    pub fn all(&self) -> &[Skill] {
        &self.skills
    }

    /// Get skills by scope
    pub fn by_scope(&self, scope: SkillScope) -> Vec<&Skill> {
        self.by_scope
            .get(&scope)
            .map(|indices| indices.iter().filter_map(|&i| self.skills.get(i)).collect())
            .unwrap_or_default()
    }

    /// Get skills matching a prompt
    pub fn matching(&self, prompt: &str) -> Vec<&Skill> {
        let prompt_lower = prompt.to_lowercase();

        self.skills
            .iter()
            .filter(|skill| {
                skill.enabled
                    && skill.triggers.iter().any(|t| prompt_lower.contains(&t.to_lowercase()))
            })
            .collect()
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.name == name)
    }

    /// Get enabled skills
    pub fn enabled(&self) -> Vec<&Skill> {
        self.skills.iter().filter(|s| s.enabled).collect()
    }

    /// Get count of skills
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Generate system prompt from all enabled skills
    pub fn generate_system_prompt(&self) -> String {
        let enabled = self.enabled();
        if enabled.is_empty() {
            return String::new();
        }

        let mut prompt = String::from("# Available Skills\n\n");
        for skill in &enabled {
            prompt.push_str(&skill.as_system_prompt());
            prompt.push_str("\n---\n\n");
        }

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
name: test-skill
description: A test skill
enabled: true
---

This is the skill content.
"#;

        let (fm, body) = parse_frontmatter(content).unwrap();
        assert_eq!(fm.get("name"), Some(&"test-skill".to_string()));
        assert_eq!(fm.get("description"), Some(&"A test skill".to_string()));
        assert_eq!(fm.get("enabled"), Some(&"true".to_string()));
        assert_eq!(body, "This is the skill content.");
    }

    #[test]
    fn test_parse_frontmatter_no_fm() {
        let content = "No frontmatter here";
        assert!(parse_frontmatter(content).is_none());
    }

    #[test]
    fn test_parse_boolean() {
        assert!(parse_boolean_frontmatter(Some(&serde_yaml::Value::Bool(true))));
        assert!(!parse_boolean_frontmatter(Some(&serde_yaml::Value::Bool(false))));
        assert!(parse_boolean_frontmatter(Some(&serde_yaml::Value::String("true".to_string()))));
        assert!(parse_boolean_frontmatter(Some(&serde_yaml::Value::String("True".to_string()))));
        assert!(!parse_boolean_frontmatter(Some(&serde_yaml::Value::String("false".to_string()))));
        assert!(!parse_boolean_frontmatter(None));
    }

    #[test]
    fn test_parse_array() {
        let yaml_seq = serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("a".to_string()),
            serde_yaml::Value::String("b".to_string()),
        ]);
        assert_eq!(parse_array_frontmatter(Some(&yaml_seq)), vec!["a", "b"]);

        let yaml_str = serde_yaml::Value::String("a, b, c".to_string());
        assert_eq!(parse_array_frontmatter(Some(&yaml_str)), vec!["a", "b", "c"]);

        assert!(parse_array_frontmatter(None).is_empty());
    }

    #[test]
    fn test_skill_scope_order() {
        assert!(SkillScope::Local < SkillScope::Repo);
        assert!(SkillScope::Repo < SkillScope::User);
        assert!(SkillScope::User < SkillScope::ClaudeCompat);
    }

    #[test]
    fn test_skill_matches() {
        let skill = Skill {
            name: "test".to_string(),
            description: None,
            path: PathBuf::from("/test/SKILL.md"),
            scope: SkillScope::Local,
            triggers: vec!["deploy".to_string(), "release".to_string()],
            enabled: true,
            frontmatter: HashMap::new(),
            content: String::new(),
        };

        assert!(skill.matches("deploy to production"));
        assert!(skill.matches("RELEASE v1.0"));
        assert!(!skill.matches("just testing"));
    }

    #[test]
    fn test_skill_as_system_prompt() {
        let skill = Skill {
            name: "Code Review".to_string(),
            description: Some("Guidelines for reviewing code".to_string()),
            path: PathBuf::from("/test/SKILL.md"),
            scope: SkillScope::Local,
            triggers: vec!["review".to_string()],
            enabled: true,
            frontmatter: HashMap::new(),
            content: "Always check for edge cases.".to_string(),
        };

        let prompt = skill.as_system_prompt();
        assert!(prompt.contains("Code Review"));
        assert!(prompt.contains("Guidelines"));
        assert!(prompt.contains("Always check for edge cases"));
    }
}
