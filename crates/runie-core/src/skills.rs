//! Skills System — Load SKILL.md files from user and project directories.
//!
//! Supports both flat layout (`name.md`) and nested layout (`name/SKILL.md`).
//! YAML frontmatter is optional and takes precedence over markdown sections.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A loaded skill parsed from a SKILL.md file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub context: String,
    pub user_invocable: bool,
    pub file_path: PathBuf,
}

impl Skill {
    /// Build a one-line summary for listing.
    pub fn summary(&self) -> String {
        let invocable = if self.user_invocable {
            " (invocable)"
        } else {
            ""
        };
        format!("{}{} — {}", self.name, invocable, self.description)
    }
}

/// Load skills from both user (`~/.runie/skills/`) and project (`./.runie/skills/`) directories.
pub fn load_all() -> Vec<Skill> {
    let mut skills = Vec::new();

    if let Some(home) = dirs::home_dir() {
        let user_dir = home.join(".runie").join("skills");
        skills.extend(load_from_dir(&user_dir));
    }

    let project_dir = PathBuf::from(".runie").join("skills");
    skills.extend(load_from_dir(&project_dir));

    skills
}

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
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unnamed");
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
fn parse_skill_md(path: &Path) -> Option<Skill> {
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
                path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
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
fn extract_frontmatter(content: &str) -> HashMap<String, String> {
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
fn extract_section(content: &str, heading: &str) -> Option<String> {
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

/// Build a combined context string from all skills that have context.
pub fn build_skills_context(skills: &[Skill]) -> String {
    let contexts: Vec<&str> = skills
        .iter()
        .map(|s| s.context.trim())
        .filter(|c| !c.is_empty())
        .collect();

    if contexts.is_empty() {
        String::new()
    } else {
        format!("\n\nAdditional context:\n{}", contexts.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn load_skills_from_dir_parses_markdown() {
        let dir = tempdir().unwrap();
        let mut file = std::fs::File::create(dir.path().join("rust.md")).unwrap();
        file.write_all(
            b"# Rust Skill\n\n## Description\n\nBest practices for Rust.\n\n## Context\n\nAlways use clippy.\n\n## Invocation\n\nUser can invoke with /skill rust\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "rust");
        assert_eq!(skills[0].description, "Best practices for Rust.");
        assert_eq!(skills[0].context, "Always use clippy.");
        assert!(skills[0].user_invocable);
    }

    #[test]
    fn skill_not_user_invocable_without_invocation_section() {
        let dir = tempdir().unwrap();
        let mut file = std::fs::File::create(dir.path().join("quiet.md")).unwrap();
        file.write_all(
            b"# Quiet\n\n## Description\n\nBe concise.\n\n## Context\n\nKeep answers short.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert!(!skills[0].user_invocable);
    }

    #[test]
    fn empty_dir_returns_no_skills() {
        let dir = tempdir().unwrap();
        let skills = load_from_dir(dir.path());
        assert!(skills.is_empty());
    }

    #[test]
    fn nonexistent_dir_returns_no_skills() {
        let skills = load_from_dir(Path::new("/does/not/exist"));
        assert!(skills.is_empty());
    }

    #[test]
    fn skill_injects_context() {
        let skills = vec![Skill {
            name: "rust".into(),
            description: "Rust best practices".into(),
            context: "Use clippy.".into(),
            user_invocable: false,
            file_path: PathBuf::from("rust.md"),
        }];
        let ctx = build_skills_context(&skills);
        assert!(ctx.contains("Use clippy."));
        assert!(ctx.contains("Additional context:"));
    }

    #[test]
    fn empty_context_returns_empty_string() {
        let skills = vec![Skill {
            name: "empty".into(),
            description: "Nothing".into(),
            context: "".into(),
            user_invocable: false,
            file_path: PathBuf::from("empty.md"),
        }];
        let ctx = build_skills_context(&skills);
        assert!(ctx.is_empty());
    }

    #[test]
    fn user_invocable_shown_in_summary() {
        let skill = Skill {
            name: "test".into(),
            description: "A test skill".into(),
            context: "".into(),
            user_invocable: true,
            file_path: PathBuf::from("test.md"),
        };
        assert!(skill.summary().contains("(invocable)"));
    }

    #[test]
    fn load_all_merges_user_and_project() {
        let user_dir = tempdir().unwrap();
        let project_dir = tempdir().unwrap();

        let mut file = std::fs::File::create(user_dir.path().join("user_skill.md")).unwrap();
        file.write_all(b"# User\n\n## Description\n\nUser skill.\n")
            .unwrap();

        let mut file = std::fs::File::create(project_dir.path().join("project_skill.md")).unwrap();
        file.write_all(b"# Project\n\n## Description\n\nProject skill.\n")
            .unwrap();

        // load_all uses hardcoded paths, so test merge manually
        let mut skills = load_from_dir(user_dir.path());
        skills.extend(load_from_dir(project_dir.path()));
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn non_md_files_are_ignored() {
        let dir = tempdir().unwrap();
        let mut file = std::fs::File::create(dir.path().join("readme.txt")).unwrap();
        file.write_all(b"## Description\n\nNot a skill.\n").unwrap();

        let skills = load_from_dir(dir.path());
        assert!(skills.is_empty());
    }

    #[test]
    fn subdirectory_skill_loads() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("rust");
        std::fs::create_dir(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        file.write_all(
            b"# Rust Skill\n\n## Description\n\nBest practices for Rust.\n\n## Context\n\nUse clippy.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "rust");
        assert_eq!(skills[0].description, "Best practices for Rust.");
    }

    #[test]
    fn subdirectory_prefers_over_flat_file() {
        let dir = tempdir().unwrap();

        // Flat file
        let mut flat = std::fs::File::create(dir.path().join("rust.md")).unwrap();
        flat.write_all(b"# Flat Rust\n\n## Description\n\nFlat description.\n")
            .unwrap();

        // Subdirectory version (should win)
        let skill_dir = dir.path().join("rust");
        std::fs::create_dir(&skill_dir).unwrap();
        let mut subdir = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        subdir.write_all(
            b"# Subdir Rust\n\n## Description\n\nSubdir description.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "rust");
        assert_eq!(skills[0].description, "Subdir description.");
    }

    #[test]
    fn yaml_frontmatter_overrides_name_and_description() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("my-skill");
        std::fs::create_dir(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        file.write_all(
            b"---\nname: custom-name\ndescription: From frontmatter\ncontext: Some context.\n---\n\n## Description\n\nFrom section.\n\n## Context\n\nSome section context.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "custom-name");
        assert_eq!(skills[0].description, "From frontmatter");
        assert_eq!(skills[0].context, "Some context.");
    }

    #[test]
    fn yaml_frontmatter_falls_back_to_sections() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("my-skill");
        std::fs::create_dir(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        file.write_all(
            b"---\ndescription: Frontmatter desc\n---\n\n## Description\n\nSection desc.\n\n## Context\n\nSome context.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        // Name from dir, description from frontmatter
        assert_eq!(skills[0].name, "my-skill");
        assert_eq!(skills[0].description, "Frontmatter desc");
        assert_eq!(skills[0].context, "Some context.");
    }

    #[test]
    fn flat_md_file_still_works() {
        let dir = tempdir().unwrap();
        let mut file = std::fs::File::create(dir.path().join("flat.md")).unwrap();
        file.write_all(
            b"# Flat Skill\n\n## Description\n\nA flat skill.\n\n## Context\n\nFlat context.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "flat");
        assert_eq!(skills[0].description, "A flat skill.");
    }

    #[test]
    fn build_skills_context_includes_subdir_skill() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("code-review");
        std::fs::create_dir(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        file.write_all(
            b"# Code Review\n\n## Description\n\nReview code.\n\n## Context\n\nRun clippy before review.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        let ctx = build_skills_context(&skills);
        assert!(ctx.contains("Run clippy before review."));
    }

    // ── serde_yaml-specific tests ────────────────────────────────────────────────

    #[test]
    fn serde_yaml_frontmatter_parses_quoted_strings() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("quoted");
        std::fs::create_dir(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        // Quoted strings: double-quoted handles colons, single-quoted preserves literal
        file.write_all(
            b"---\nname: \"quoted-name\"\ndescription: \"Desc with colon: inside\"\ncontext: 'Context with single quotes'\n---\n\n## Description\n\nNot used.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "quoted-name");
        assert_eq!(skills[0].description, "Desc with colon: inside");
        assert_eq!(skills[0].context, "Context with single quotes");
    }

    #[test]
    fn serde_yaml_frontmatter_parses_multiline_context() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("multiline");
        std::fs::create_dir(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        // Multiline using | (literal block scalar)
        file.write_all(
            b"---\nname: multiline-skill\ndescription: A skill\ncontext: |\n  Line one\n  Line two\n  Line three\n---\n\n## Description\n\nNot used.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].context, "Line one\nLine two\nLine three");
    }

    #[test]
    fn serde_yaml_frontmatter_parses_multiline_with_indentation() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("folded");
        std::fs::create_dir(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        // Multiline using > (folded block scalar)
        file.write_all(
            b"---\nname: folded-skill\ndescription: A skill\ncontext: >\n  This is\n  folded into\n  a single line\n---\n\n## Description\n\nNot used.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        // Folded scalars replace newlines with spaces
        assert!(skills[0].context.contains("folded into"));
    }

    #[test]
    fn serde_yaml_frontmatter_ignores_non_string_values() {
        let dir = tempdir().unwrap();
        let skill_dir = dir.path().join("mixed");
        std::fs::create_dir(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        // A list value (not a string) should be ignored, plain string should be kept
        file.write_all(
            b"---\nname: mixed-skill\ntags:\n  - rust\n  - tool\ndescription: Plain string description\n---\n\n## Description\n\nNot used.\n",
        )
        .unwrap();

        let skills = load_from_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "mixed-skill");
        assert_eq!(skills[0].description, "Plain string description");
    }

    #[test]
    fn serde_yaml_frontmatter_no_frontmatter_returns_empty() {
        let fm = extract_frontmatter("# No frontmatter\n\n## Description\n\nJust text.\n");
        assert!(fm.is_empty());
    }

    #[test]
    fn serde_yaml_frontmatter_empty_frontmatter_returns_empty() {
        let fm = extract_frontmatter("---\n---\n\n## Description\n\nNo keys.\n");
        assert!(fm.is_empty());
    }

    #[test]
    fn serde_yaml_frontmatter_single_quoted_values() {
        let fm = extract_frontmatter("---\nname: 'single quoted'\ndescription: 'also single'\n---\n");
        assert_eq!(fm.get("name"), Some(&"single quoted".to_string()));
        assert_eq!(fm.get("description"), Some(&"also single".to_string()));
    }
}
