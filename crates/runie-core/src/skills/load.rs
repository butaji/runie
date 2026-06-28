//! Skill loading from directory paths.
//!
//! Supports both subdirectory skills (SKILL.md files) and flat .md files.

use std::path::Path;

#[cfg(test)]
use crate::resource_loader::parse_resource_md;
use crate::resource_loader::{
    extract_section, is_user_invocable, load_resources_from_dir, resolve_name,
};

use super::Skill;

/// Load all skills from a directory.
/// Scans for subdirectories with SKILL.md files first, then flat .md files.
/// Subdirectory skills take precedence over flat files with the same name.
pub fn load_from_dir(dir: &Path) -> Vec<Skill> {
    let records = load_resources_from_dir(dir);
    records
        .into_iter()
        .filter_map(record_to_skill)
        .collect()
}

/// Convert a ResourceRecord to a Skill, with markdown fallback for missing fields.
fn record_to_skill(record: crate::resource_loader::ResourceRecord) -> Option<Skill> {
    let name = resolve_name(&record.file_path, &record.frontmatter);

    // Use frontmatter with markdown section fallback
    let description = record
        .frontmatter
        .get("description")
        .cloned()
        .unwrap_or_else(|| {
            extract_section(&record.content, "Description").unwrap_or_default()
        });
    let context = record
        .frontmatter
        .get("context")
        .cloned()
        .unwrap_or_else(|| {
            extract_section(&record.content, "Context").unwrap_or_default()
        });
    let invocation = record
        .frontmatter
        .get("invocation")
        .cloned()
        .unwrap_or_else(|| {
            extract_section(&record.content, "Invocation").unwrap_or_default()
        });

    let user_invocable = is_user_invocable(&invocation);

    Some(Skill {
        name,
        description,
        context,
        user_invocable,
        file_path: record.file_path,
    })
}

/// Parse a single SKILL.md file, optionally with YAML frontmatter.
/// Uses markdown section fallback for missing frontmatter fields.
#[cfg(test)]
pub(crate) fn parse_skill_md(path: &Path) -> Option<Skill> {
    let record = parse_resource_md(path)?;
    record_to_skill(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn load_from_dir_with_subdir_skill() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        std::fs::create_dir(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: my-skill
description: A test skill
---

# Body
"#,
        )
        .unwrap();

        let skills = load_from_dir(tmp.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "my-skill");
        assert_eq!(skills[0].description, "A test skill");
    }

    #[test]
    fn load_from_dir_with_markdown_fallback() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("fallback-skill");
        std::fs::create_dir(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: fallback-skill
---

## Description

Description from markdown section.

## Context

Context from markdown section.
"#,
        )
        .unwrap();

        let skills = load_from_dir(tmp.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "fallback-skill");
        assert_eq!(skills[0].description, "Description from markdown section.");
        assert_eq!(skills[0].context, "Context from markdown section.");
    }

    #[test]
    fn parse_skill_md_extracts_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test-skill.md");
        std::fs::write(
            &path,
            r#"---
name: test-skill
description: Test description
context: Test context
invocation: user can invoke
---

# Test Skill
"#,
        )
        .unwrap();

        let skill = parse_skill_md(&path).unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "Test description");
        assert_eq!(skill.context, "Test context");
        assert!(skill.user_invocable);
    }

    #[test]
    fn subdir_precedence_over_flat() {
        let tmp = TempDir::new().unwrap();

        // Create subdir skill
        let skill_dir = tmp.path().join("shared-name");
        std::fs::create_dir(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: subdir-skill
description: From subdirectory
---
"#,
        )
        .unwrap();

        // Create flat skill with same name stem
        std::fs::write(
            tmp.path().join("shared-name.md"),
            r#"---
name: flat-skill
description: From flat file
---
"#,
        )
        .unwrap();

        let skills = load_from_dir(tmp.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "subdir-skill");
    }
}
