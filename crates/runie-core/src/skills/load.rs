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

    // Track which skill names we've loaded from subdirectories
    let mut subdir_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut skills = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        // Check for subdirectory with SKILL.md
        if path.is_dir() {
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    subdir_names.insert(name.to_string());
                }
                if let Some(skill) = parse_skill_md(&skill_md) {
                    skills.push(skill);
                }
            }
        }
    }

    // Now scan for flat .md files, skipping those already loaded from subdirectories
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return skills,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unnamed");
            if !subdir_names.contains(stem) {
                if let Some(skill) = parse_skill_md(&path) {
                    skills.push(skill);
                }
            }
        }
    }

    skills
}

/// Parse a single SKILL.md file, optionally with YAML frontmatter.
pub(crate) fn parse_skill_md(path: &Path) -> Option<Skill> {
    let content = std::fs::read_to_string(path).ok()?;

    // Parse optional YAML frontmatter using serde_yaml
    let frontmatter = extract_frontmatter(&content);

    let name = frontmatter
        .get("name")
        .cloned()
        .or_else(|| {
            // Derive name from path: for subdir/SKILL.md use dir name, else file stem
            if path.file_name().and_then(|s| s.to_str()) == Some("SKILL.md") {
                path.parent()?.file_name()?.to_str().map(|s| s.to_string())
            } else {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            }
        })
        .unwrap_or_else(|| "unnamed".to_string());

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
    let user_invocable = invocation.to_lowercase().contains("user can invoke")
        || invocation.to_lowercase().contains("/skill");

    Some(Skill {
        name,
        description,
        context,
        user_invocable,
        file_path: path.to_owned(),
    })
}

/// Extract YAML frontmatter from content if present, using serde_yaml.
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
    // Body starts after the closing "---\n"
    let _body = &after_opening[end_pos + 4..];

    // Parse YAML with serde_yaml
    match serde_yaml::from_str::<serde_yaml::Value>(fm_text) {
        Ok(serde_yaml::Value::Mapping(mapping)) => {
            let mut result = HashMap::new();
            for (k, v) in mapping {
                if let serde_yaml::Value::String(key) = k {
                    if let serde_yaml::Value::String(val) = v {
                        result.insert(key, val);
                    }
                }
            }
            result
        }
        // Non-mapping values (e.g. a bare string) are treated as no frontmatter
        _ => HashMap::new(),
    }
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
        Some(text.to_string())
    }
}
