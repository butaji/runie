//! Generic loader for declarative configuration files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::types::{CommandCategory, CommandDef, SkillDef, Trigger};

/// Generic loader for declarative configuration.
pub struct DeclarativeLoader {
    /// Directories to search for skills (markdown files).
    skill_dirs: Vec<PathBuf>,
    /// Directories to search for commands (yaml files).
    command_dirs: Vec<PathBuf>,
}

impl DeclarativeLoader {
    /// Create a new loader with the given directories.
    pub fn new() -> Self {
        Self {
            skill_dirs: Vec::new(),
            command_dirs: Vec::new(),
        }
    }

    /// Add a directory to search for skills.
    pub fn with_skill_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.skill_dirs.push(dir.into());
        self
    }

    /// Add a directory to search for commands.
    pub fn with_command_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.command_dirs.push(dir.into());
        self
    }

    /// Load all skills from configured directories.
    pub fn load_skills(&self) -> Vec<SkillDef> {
        let mut skills = Vec::new();
        for dir in &self.skill_dirs {
            skills.extend(load_skills_from_dir(dir));
        }
        skills
    }

    /// Load all commands from configured directories.
    pub fn load_commands(&self) -> Vec<CommandDef> {
        let mut commands = Vec::new();
        for dir in &self.command_dirs {
            commands.extend(load_commands_from_dir(dir));
        }
        commands
    }
}

impl Default for DeclarativeLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Skill loading
// ---------------------------------------------------------------------------

/// Load all skills from a directory.
pub fn load_skills_from_dir(dir: &Path) -> Vec<SkillDef> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let subdir_skills = load_subdir_skills(entries);
    let subdir_names: HashMap<String, ()> = subdir_skills
        .iter()
        .map(|s| (s.name.clone(), ()))
        .collect();

    let flat_skills = load_flat_skills(dir, &subdir_names);
    subdir_skills
        .into_iter()
        .chain(flat_skills)
        .collect()
}

fn load_subdir_skills(entries: std::fs::ReadDir) -> Vec<SkillDef> {
    let mut skills = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if skill_md.exists() {
            if let Some(skill) = parse_skill_md(&skill_md) {
                skills.push(skill);
            }
        }
    }
    skills
}

fn load_flat_skills(dir: &Path, subdir_names: &HashMap<String, ()>) -> Vec<SkillDef> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut skills = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        // Skip SKILL.md files and files matching subdirectory names
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if file_name == "SKILL.md" || subdir_names.contains_key(file_name.trim_end_matches(".md")) {
            continue;
        }
        if let Some(skill) = parse_skill_md(&path) {
            skills.push(skill);
        }
    }
    skills
}

/// Load all commands from a directory.
pub fn load_commands_from_dir(dir: &Path) -> Vec<CommandDef> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut commands = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            if let Some(cmd) = parse_command_yaml(&path) {
                commands.push(cmd);
            }
        }
    }
    commands
}

/// Parse a skill markdown file with YAML frontmatter.
pub(crate) fn parse_skill_md(path: &Path) -> Option<SkillDef> {
    let content = std::fs::read_to_string(path).ok()?;
    let frontmatter = extract_frontmatter(&content)?;

    let name = frontmatter
        .get("name")
        .cloned()
        .or_else(|| derive_name_from_path(path))?;

    Some(SkillDef {
        name,
        description: frontmatter.get("description").cloned().unwrap_or_default(),
        context: frontmatter.get("context").cloned(),
        triggers: parse_triggers(&frontmatter),
        file_path: path.to_owned(),
        user_invocable: is_user_invocable(
            frontmatter.get("invocation").cloned().unwrap_or_default(),
        ),
    })
}

// ---------------------------------------------------------------------------
// Command loading
// ---------------------------------------------------------------------------

/// Parse a command YAML file.
pub(crate) fn parse_command_yaml(path: &Path) -> Option<CommandDef> {
    let content = std::fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    let map = value.as_mapping()?;
    let name = get_string(map, "name")?;

    Some(CommandDef {
        name,
        description: get_string(map, "description").unwrap_or_default(),
        category: get_string(map, "category")
            .map(|s| CommandCategory::parse(&s))
            .unwrap_or_default(),
        intent: get_string(map, "intent").unwrap_or_default(),
        shortcut: get_string(map, "shortcut"),
        has_subcommands: get_bool(map, "subcommands").unwrap_or(false),
        file_path: path.to_owned(),
    })
}

// ---------------------------------------------------------------------------
// Frontmatter parsing
// ---------------------------------------------------------------------------

/// Extract YAML frontmatter from markdown content.
pub(crate) fn extract_frontmatter(content: &str) -> Option<HashMap<String, String>> {
    if !content.starts_with("---\n") {
        return None;
    }
    let after_opening = content.strip_prefix("---\n")?;
    let end_pos = after_opening.find("\n---")?;
    let fm_text = &after_opening[..end_pos];
    Some(parse_frontmatter_yaml(fm_text))
}

/// Parse simple YAML frontmatter: "key: value" lines.
fn parse_frontmatter_yaml(fm_text: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for line in fm_text.lines() {
        if let Some((k, v)) = parse_yaml_line(line) {
            result.insert(k, v);
        }
    }
    result
}

/// Parse a single YAML "key: value" line.
pub(crate) fn parse_yaml_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let colon_pos = line.find(':')?;
    let key = line[..colon_pos].trim().to_owned();
    if key.is_empty() {
        return None;
    }
    Some((key, strip_quotes(line[colon_pos + 1..].trim())))
}

/// Strip surrounding quotes from a YAML value.
pub(crate) fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        s[1..s.len() - 1].to_owned()
    } else {
        s.to_owned()
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Derive skill name from file path.
fn derive_name_from_path(path: &Path) -> Option<String> {
    if path.file_name().and_then(|s| s.to_str()) == Some("SKILL.md") {
        path.parent()?.file_name()?.to_str().map(String::from)
    } else {
        path.file_stem()?.to_str().map(String::from)
    }
}

/// Check if invocation string indicates user can invoke.
fn is_user_invocable(invocation: String) -> bool {
    let lower = invocation.to_lowercase();
    lower.contains("user can invoke") || lower.contains("/skill")
}

/// Parse triggers from frontmatter.
pub(crate) fn parse_triggers(frontmatter: &HashMap<String, String>) -> Vec<Trigger> {
    let mut triggers = Vec::new();
    triggers.extend(parse_command_triggers(frontmatter));
    triggers.extend(parse_single_command(frontmatter));
    triggers
}

fn parse_command_triggers(frontmatter: &HashMap<String, String>) -> Vec<Trigger> {
    let Some(triggers_str) = frontmatter.get("triggers") else {
        return Vec::new();
    };
    triggers_str.lines().filter_map(parse_trigger_line).collect()
}

fn parse_trigger_line(line: &str) -> Option<Trigger> {
    let line = line.trim();
    if line.starts_with("- command:") {
        line.strip_prefix("- command:")
            .map(|s| Trigger::Command(s.trim().trim_matches('"').trim_matches('\'').to_owned()))
    } else if line.starts_with('-') {
        Trigger::parse(line.trim_start_matches('-').trim())
    } else if !line.is_empty() {
        Trigger::parse(line)
    } else {
        None
    }
}

fn parse_single_command(frontmatter: &HashMap<String, String>) -> Vec<Trigger> {
    frontmatter
        .get("command")
        .map(|cmd| vec![Trigger::Command(cmd.clone())])
        .unwrap_or_default()
}

/// Get a string value from a YAML mapping.
fn get_string(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
    let key_value = serde_yaml::Value::String(key.to_owned());
    map.get(&key_value)?.as_str().map(String::from)
}

/// Get a boolean value from a YAML mapping.
fn get_bool(map: &serde_yaml::Mapping, key: &str) -> Option<bool> {
    let key_value = serde_yaml::Value::String(key.to_owned());
    map.get(&key_value)?
        .as_bool()
        .or_else(|| map.get(&key_value)?.as_str().map(|s| s == "true"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter_basic() {
        let content = r#"---
name: test-skill
description: A test skill
---

# Body content
"#;
        let fm = extract_frontmatter(content).unwrap();
        assert_eq!(fm.get("name"), Some(&"test-skill".to_owned()));
    }

    #[test]
    fn parse_triggers_list() {
        let mut fm = HashMap::new();
        fm.insert(
            "triggers".to_owned(),
            "- command: /check-work\n- command: /verify".to_owned(),
        );
        let triggers = parse_triggers(&fm);
        assert_eq!(triggers.len(), 2);
        assert_eq!(triggers[0], Trigger::Command("/check-work".to_owned()));
    }

    #[test]
    fn derive_name_from_skill_md() {
        assert_eq!(
            derive_name_from_path(Path::new("/path/to/my-skill/SKILL.md")).unwrap(),
            "my-skill"
        );
    }

    #[test]
    fn is_user_invocable_checks_keyword() {
        assert!(is_user_invocable("user can invoke this skill".to_owned()));
        assert!(is_user_invocable("Try /skill name".to_owned()));
        assert!(!is_user_invocable("automatic".to_owned()));
    }
}
