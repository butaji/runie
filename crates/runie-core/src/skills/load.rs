use std::collections::HashMap;
use std::path::Path;

use super::Skill;

/// Load all skills from a directory.
/// Scans for subdirectories with SKILL.md files first, then flat .md files.
/// Subdirectory skills take precedence over flat files with the same name.
pub fn load_from_dir(dir: &Path) -> Vec<Skill> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let (subdir_skills, subdir_names) = load_subdir_skills(entries);
    let mut skills = subdir_skills;

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return skills,
    };
    load_flat_skills(entries, &subdir_names, &mut skills);
    skills
}

fn load_subdir_skills(
    entries: std::fs::ReadDir,
) -> (Vec<Skill>, std::collections::HashSet<String>) {
    let mut names = std::collections::HashSet::new();
    let mut skills = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            names.insert(name.to_owned());
        }
        if let Some(skill) = parse_skill_md(&skill_md) {
            skills.push(skill);
        }
    }
    (skills, names)
}

fn load_flat_skills(
    entries: std::fs::ReadDir,
    subdir_names: &std::collections::HashSet<String>,
    skills: &mut Vec<Skill>,
) {
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed");
        if subdir_names.contains(stem) {
            continue;
        }
        if let Some(skill) = parse_skill_md(&path) {
            skills.push(skill);
        }
    }
}

/// Parse a single SKILL.md file, optionally with YAML frontmatter.
pub(crate) fn parse_skill_md(path: &Path) -> Option<Skill> {
    let content = std::fs::read_to_string(path).ok()?;
    let frontmatter = extract_frontmatter(&content);
    let name = resolve_skill_name(path, &frontmatter);
    let description = frontmatter
        .get("description")
        .cloned()
        .unwrap_or_else(|| extract_section(&content, "Description").unwrap_or_default());
    let context = frontmatter
        .get("context")
        .cloned()
        .unwrap_or_else(|| extract_section(&content, "Context").unwrap_or_default());
    let invocation = frontmatter
        .get("invocation")
        .cloned()
        .unwrap_or_else(|| extract_section(&content, "Invocation").unwrap_or_default());
    let user_invocable = is_user_invocable(&invocation);

    Some(Skill {
        name,
        description,
        context,
        user_invocable,
        file_path: path.to_owned(),
    })
}

fn resolve_skill_name(path: &Path, frontmatter: &HashMap<String, String>) -> String {
    frontmatter
        .get("name")
        .cloned()
        .unwrap_or_else(|| derive_skill_name(path).unwrap_or_else(|| "unnamed".to_owned()))
}

fn derive_skill_name(path: &Path) -> Option<String> {
    if path.file_name().and_then(|s| s.to_str()) == Some("SKILL.md") {
        path.parent()?.file_name()?.to_str().map(|s| s.to_owned())
    } else {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_owned())
    }
}

fn is_user_invocable(invocation: &str) -> bool {
    let lower = invocation.to_lowercase();
    lower.contains("user can invoke") || lower.contains("/skill")
}

/// Parse simple YAML frontmatter: "key: value" lines.
/// Supports unquoted values and single-quoted values. No nested structures.
fn parse_frontmatter_yaml(fm_text: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for line in fm_text.lines() {
        if let Some((key, value)) = parse_yaml_line(line) {
            result.insert(key, value);
        }
    }
    result
}

/// Parse a single YAML "key: value" line, returning (key, value) or None.
fn parse_yaml_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let colon_pos = line.find(':')?;
    let key = line[..colon_pos].trim().to_owned();
    if key.is_empty() {
        return None;
    }
    let value = strip_quotes(line[colon_pos + 1..].trim());
    Some((key, value))
}

/// Strip surrounding quotes from a YAML value.
fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    let unquoted = if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        &s[1..s.len() - 1]
    } else {
        s
    };
    unquoted.to_owned()
}

/// Extract YAML frontmatter from content if present.
/// Returns a HashMap of key-value pairs suitable for the skill frontmatter schema.
pub(crate) fn extract_frontmatter(content: &str) -> HashMap<String, String> {
    // Only recognize frontmatter if content starts with "---"
    if !content.starts_with("---\n") {
        return HashMap::new();
    }

    // Find the closing "---"
    let after_opening = match content.strip_prefix("---\n") {
        Some(s) => s,
        None => return HashMap::new(),
    };

    let end_pos = match after_opening.find("\n---") {
        Some(p) => p,
        None => return HashMap::new(),
    };

    let fm_text = &after_opening[..end_pos];

    parse_frontmatter_yaml(fm_text)
}

/// Extract text under a markdown `## Section` heading.
pub(crate) fn extract_section(content: &str, heading: &str) -> Option<String> {
    let search = format!("## {}", heading);
    let start = content.find(&search)?;
    let after_heading = &content[start + search.len()..];

    // Find the next ## heading or end of file
    let end = after_heading.find("\n## ").unwrap_or(after_heading.len());

    let text = after_heading[..end].trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}
