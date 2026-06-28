//! Generic loader for declarative configuration files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::resource_loader::{
    derive_name_from_path, is_user_invocable, load_resources_from_dir,
};

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
    let records = load_resources_from_dir(dir);
    records
        .into_iter()
        .filter_map(record_to_skill_def)
        .collect()
}

/// Convert a ResourceRecord to a SkillDef.
/// Unlike skills/load.rs, this requires frontmatter (no markdown fallback).
fn record_to_skill_def(record: crate::resource_loader::ResourceRecord) -> Option<SkillDef> {
    let frontmatter = record.frontmatter;

    let name = frontmatter
        .get("name")
        .cloned()
        .or_else(|| derive_name_from_path(&record.file_path))?;

    let invocation = frontmatter.get("invocation").cloned().unwrap_or_default();

    Some(SkillDef {
        name,
        description: frontmatter.get("description").cloned().unwrap_or_default(),
        context: frontmatter.get("context").cloned(),
        triggers: parse_triggers(&frontmatter),
        file_path: record.file_path,
        user_invocable: is_user_invocable(&invocation),
    })
}

// ---------------------------------------------------------------------------
// Command loading
// ---------------------------------------------------------------------------

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
// Trigger parsing
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// YAML helpers (command parsing only)
// ---------------------------------------------------------------------------

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
        assert!(is_user_invocable("user can invoke this skill"));
        assert!(is_user_invocable("Try /skill name"));
        assert!(!is_user_invocable("automatic"));
    }
}
