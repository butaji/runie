//! Shared resource loader for markdown-based declarative resources.
//!
//! Provides directory scanning, YAML frontmatter extraction, and name resolution
//! for skills and other markdown-based resources.

use pulldown_cmark_frontmatter::FrontmatterExtractor;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A generic resource record parsed from a markdown file.
#[derive(Debug, Clone)]
pub struct ResourceRecord {
    /// Extracted frontmatter key-value pairs.
    pub frontmatter: HashMap<String, String>,
    /// Raw markdown content (without frontmatter).
    pub content: String,
    /// Path to the source file.
    pub file_path: PathBuf,
}

/// Load all markdown resources from a directory.
/// Scans for subdirectories with SKILL.md files first, then flat .md files.
/// Subdirectory resources take precedence over flat files with the same name.
pub fn load_resources_from_dir(dir: &Path) -> Vec<ResourceRecord> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let (subdir_records, subdir_names) = load_subdir_resources(entries);
    let mut records = subdir_records;

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return records,
    };
    load_flat_resources(entries, &subdir_names, &mut records);
    records
}

fn load_subdir_resources(
    entries: fs::ReadDir,
) -> (Vec<ResourceRecord>, std::collections::HashSet<String>) {
    let mut names = std::collections::HashSet::new();
    let mut records = Vec::new();

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
        if let Some(record) = parse_resource_md(&skill_md) {
            records.push(record);
        }
    }
    (records, names)
}

fn load_flat_resources(
    entries: fs::ReadDir,
    subdir_names: &std::collections::HashSet<String>,
    records: &mut Vec<ResourceRecord>,
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
        if let Some(record) = parse_resource_md(&path) {
            records.push(record);
        }
    }
}

/// Parse a markdown file with optional YAML frontmatter.
pub fn parse_resource_md(path: &Path) -> Option<ResourceRecord> {
    let content = fs::read_to_string(path).ok()?;
    let frontmatter = extract_frontmatter(&content);
    let file_path = path.to_owned();
    Some(ResourceRecord {
        frontmatter,
        content,
        file_path,
    })
}

/// Extract YAML frontmatter from markdown content if present.
/// Returns a HashMap of key-value pairs. Returns empty map if no frontmatter.
pub fn extract_frontmatter(content: &str) -> HashMap<String, String> {
    // pulldown-cmark-frontmatter expects fenced code blocks for frontmatter.
    // Normalize raw --- delimiters to fenced blocks if needed.
    let normalized = normalize_raw_frontmatter(content);
    let extractor = FrontmatterExtractor::from_markdown(&normalized);
    match extractor.extract() {
        Some(fm) => {
            if let Some(code_block) = fm.code_block {
                yaml_frontmatter_to_hashmap(&code_block.source)
            } else {
                HashMap::new()
            }
        }
        None => HashMap::new(),
    }
}

/// Normalize raw `---` frontmatter delimiters to fenced code blocks.
/// pulldown-cmark-frontmatter expects fenced code blocks.
fn normalize_raw_frontmatter(content: &str) -> String {
    // Check if content uses raw --- delimiters
    if content.starts_with("---\n") {
        let after_opening = match content.strip_prefix("---\n") {
            Some(s) => s,
            None => return content.to_string(),
        };
        let end_pos = match after_opening.find("\n---") {
            Some(p) => p,
            None => return content.to_string(),
        };
        let frontmatter = &after_opening[..end_pos];
        let body = &after_opening[end_pos + 5..]; // skip "\n---"

        // Re-wrap as a fenced code block
        format!("```yaml\n{frontmatter}\n```\n{body}")
    } else {
        content.to_string()
    }
}

/// Parse YAML frontmatter text into a HashMap.
fn yaml_frontmatter_to_hashmap(source: &str) -> HashMap<String, String> {
    match serde_yaml::from_str::<Value>(source) {
        Ok(value) => yaml_value_to_hashmap(&value),
        Err(_) => HashMap::new(),
    }
}

/// Convert serde_yaml::Value to HashMap<String, String>.
/// Handles simple string values and converts other types to their debug representation.
fn yaml_value_to_hashmap(value: &Value) -> HashMap<String, String> {
    let mut result = HashMap::new();
    if let Some(map) = value.as_mapping() {
        for (k, v) in map {
            let key = k.as_str().unwrap_or_default().to_string();
            let val = match v {
                Value::String(s) => s.clone(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                _ => format!("{:?}", v),
            };
            result.insert(key, val);
        }
    }
    result
}

/// Parse a single YAML "key: value" line, returning (key, value) or None.
pub fn parse_yaml_line(line: &str) -> Option<(String, String)> {
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
pub fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    let unquoted = if (s.starts_with('\'') && s.ends_with('\''))
        || (s.starts_with('"') && s.ends_with('"'))
    {
        &s[1..s.len() - 1]
    } else {
        s
    };
    unquoted.to_owned()
}

/// Resolve resource name from path and frontmatter.
/// Prefers frontmatter "name" field, then derives from path.
pub fn resolve_name(path: &Path, frontmatter: &HashMap<String, String>) -> String {
    frontmatter
        .get("name")
        .cloned()
        .unwrap_or_else(|| derive_name_from_path(path).unwrap_or_else(|| "unnamed".to_owned()))
}

/// Derive resource name from file path.
pub fn derive_name_from_path(path: &Path) -> Option<String> {
    if path.file_name().and_then(|s| s.to_str()) == Some("SKILL.md") {
        path.parent()?.file_name()?.to_str().map(String::from)
    } else {
        path.file_stem()?.to_str().map(String::from)
    }
}

/// Check if invocation string indicates user can invoke.
pub fn is_user_invocable(invocation: &str) -> bool {
    let lower = invocation.to_lowercase();
    lower.contains("user can invoke") || lower.contains("/skill")
}

/// Extract text under a markdown `## Section` heading.
pub fn extract_section(content: &str, heading: &str) -> Option<String> {
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

/// Extract markdown body — everything after the closing `---` frontmatter marker.
/// If no frontmatter is present, returns the trimmed content.
pub fn extract_body(content: &str) -> String {
    if let Some(pos) = content.find("\n---\n") {
        content[pos + 5..].trim().to_owned()
    } else {
        content.trim().to_owned()
    }
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
        let fm = extract_frontmatter(content);
        assert_eq!(fm.get("name"), Some(&"test-skill".to_owned()));
        assert_eq!(
            fm.get("description"),
            Some(&"A test skill".to_owned())
        );
    }

    #[test]
    fn extract_frontmatter_returns_none_without_delimiters() {
        let content = "# Just a heading\n\nSome text";
        assert!(extract_frontmatter(content).is_empty());
    }

    #[test]
    fn parse_yaml_line_with_quotes() {
        assert_eq!(
            parse_yaml_line("name: 'quoted value'"),
            Some(("name".to_owned(), "quoted value".to_owned()))
        );
        assert_eq!(
            parse_yaml_line("name: \"double quoted\""),
            Some(("name".to_owned(), "double quoted".to_owned()))
        );
        assert_eq!(
            parse_yaml_line("name: unquoted"),
            Some(("name".to_owned(), "unquoted".to_owned()))
        );
    }

    #[test]
    fn strip_quotes_handles_both_styles() {
        assert_eq!(strip_quotes("'single quoted'"), "single quoted");
        assert_eq!(strip_quotes("\"double quoted\""), "double quoted");
        assert_eq!(strip_quotes("no quotes"), "no quotes");
    }

    #[test]
    fn resolve_name_prefers_frontmatter() {
        let mut fm = HashMap::new();
        fm.insert("name".to_owned(), "from-frontmatter".to_owned());
        let name = resolve_name(Path::new("/path/to/skill/SKILL.md"), &fm);
        assert_eq!(name, "from-frontmatter");
    }

    #[test]
    fn resolve_name_falls_back_to_path() {
        let fm = HashMap::new();
        let name = resolve_name(Path::new("/path/to/my-skill/SKILL.md"), &fm);
        assert_eq!(name, "my-skill");
    }

    #[test]
    fn derive_name_from_skill_md() {
        assert_eq!(
            derive_name_from_path(Path::new("/path/to/my-skill/SKILL.md")).unwrap(),
            "my-skill"
        );
    }

    #[test]
    fn derive_name_from_flat_md() {
        assert_eq!(
            derive_name_from_path(Path::new("/path/to/my-skill.md")).unwrap(),
            "my-skill"
        );
    }

    #[test]
    fn is_user_invocable_checks_keyword() {
        assert!(is_user_invocable("user can invoke this skill"));
        assert!(is_user_invocable("Try /skill name"));
        assert!(!is_user_invocable("automatic"));
    }

    #[test]
    fn extract_section_finds_heading() {
        let content = r#"## Description

This is the description.

## Context

This is context."#;
        assert_eq!(
            extract_section(content, "Description").unwrap(),
            "This is the description."
        );
    }

    #[test]
    fn extract_section_handles_missing_heading() {
        let content = "## Other\n\nContent";
        assert!(extract_section(content, "Description").is_none());
    }

    #[test]
    fn extract_body_with_frontmatter() {
        let content = r#"---
name: test
---

Body content here.
"#;
        assert_eq!(extract_body(content), "Body content here.");
    }

    #[test]
    fn extract_body_without_frontmatter() {
        let content = "Just body content.";
        assert_eq!(extract_body(content), "Just body content.");
    }

    #[test]
    fn extract_body_multiline() {
        let content = r#"---
name: multi
---

First paragraph.

Second paragraph.

Third paragraph.
"#;
        assert_eq!(
            extract_body(content),
            "First paragraph.\n\nSecond paragraph.\n\nThird paragraph."
        );
    }
}
