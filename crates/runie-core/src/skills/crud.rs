//! Skill CRUD file operations (create / delete / update).
//!
//! Writes go to the user skills directory (`~/.runie/skills/`), the same
//! location `runie skill create/delete` uses. Both flat (`name.md`) and nested
//! (`name/SKILL.md`) layouts are supported for deletion; creation always uses
//! the flat layout with a standard frontmatter template.

use std::path::{Path, PathBuf};

/// Name validation: non-empty, no path separators, no `.` / `..`.
pub fn validate_name(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Skill name cannot be empty.".into());
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!("Invalid skill name: '{name}'"));
    }
    Ok(())
}

/// Resolve the user skills directory, honoring `$HOME` first (mirrors
/// [`super::load_all`]).
pub fn user_skills_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".runie")
        .join("skills")
}

/// Create a new skill from the standard template. Errors if a skill with the
/// same name already exists (flat or nested). Returns the created file path.
pub fn create_skill(name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    validate_name(name)?;

    let dir = user_skills_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;

    let path = dir.join(format!("{name}.md"));
    if path.exists() || dir.join(name).join("SKILL.md").exists() {
        return Err(format!("Skill '{name}' already exists."));
    }

    let template = format!(
        r#"---
name: {name}
description: Description of the {name} skill
context: |
  Describe how this skill should be used and what context it provides.
---

# {name}

Describe what this skill does and how it helps.

## Usage

Describe how to use this skill.

## Examples

Provide examples of how this skill is used.
"#
    );
    std::fs::write(&path, template).map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/// Delete a skill, matching either flat (`name.md`) or nested (`name/SKILL.md`)
/// layouts. Returns the path that was removed.
pub fn delete_skill(name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    validate_name(name)?;

    let dir = user_skills_dir();
    let flat = dir.join(format!("{name}.md"));
    let nested_dir = dir.join(name);
    let nested_md = nested_dir.join("SKILL.md");

    let target = if flat.exists() {
        flat
    } else if nested_md.exists() {
        nested_dir
    } else {
        return Err(format!("Skill '{name}' not found."));
    };

    if target.is_dir() {
        std::fs::remove_dir_all(&target).map_err(|e| format!("Cannot delete {}: {e}", target.display()))?;
    } else {
        std::fs::remove_file(&target).map_err(|e| format!("Cannot delete {}: {e}", target.display()))?;
    }
    Ok(target)
}

/// Overwrite a skill's content (flat layout only). Creates the file if it does
/// not exist. Returns the written path.
pub fn update_skill(name: &str, content: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    validate_name(name)?;

    let dir = user_skills_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;

    let path = dir.join(format!("{name}.md"));
    std::fs::write(&path, content).map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/// Locate an existing skill file (flat or nested) for the given name.
/// Used by the delete handler and diagnostics.
pub fn find_skill_file(name: &str) -> Option<PathBuf> {
    let name = name.trim();
    let dir = user_skills_dir();
    let flat = dir.join(format!("{name}.md"));
    if flat.exists() {
        return Some(flat);
    }
    let nested_md = dir.join(name).join("SKILL.md");
    nested_md.exists().then_some(nested_md)
}

/// True if `path` is a plausible skill file location under the user skills dir.
pub fn is_skill_path(path: &Path) -> bool {
    let dir = user_skills_dir();
    path.starts_with(&dir)
        && (path.extension().and_then(|e| e.to_str()) == Some("md")
            || path.file_name().and_then(|f| f.to_str()) == Some("SKILL.md"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_testing::env_lock::with_env;
    use tempfile::TempDir;

    fn temp_home() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn validate_rejects_empty_and_traversal() {
        assert!(validate_name("").is_err());
        assert!(validate_name("  ").is_err());
        assert!(validate_name("a/b").is_err());
        assert!(validate_name("..").is_err());
        assert!(validate_name("a\\b").is_err());
        assert!(validate_name("valid-name").is_ok());
    }

    #[test]
    fn create_writes_template_and_delete_removes_flat() {
        with_env(|env| {
            let home = temp_home();
            env.set("HOME", home.path());

            let path = create_skill("my-skill").expect("create should succeed");
            assert!(path.ends_with(".runie/skills/my-skill.md"));
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(content.contains("name: my-skill"));

            // Duplicate creation fails.
            assert!(create_skill("my-skill").is_err());

            let removed = delete_skill("my-skill").expect("delete should succeed");
            assert_eq!(removed, path);
            assert!(!path.exists());
        });
    }

    #[test]
    fn delete_matches_nested_layout() {
        with_env(|env| {
            let home = temp_home();
            env.set("HOME", home.path());

            let dir = user_skills_dir();
            std::fs::create_dir_all(dir.join("nested")).unwrap();
            std::fs::write(dir.join("nested/SKILL.md"), "# Nested\n").unwrap();

            let removed = delete_skill("nested").expect("delete nested should succeed");
            assert!(removed.ends_with("nested"));
            assert!(!dir.join("nested").exists());
        });
    }

    #[test]
    fn delete_unknown_errors() {
        with_env(|env| {
            let home = temp_home();
            env.set("HOME", home.path());
            assert!(delete_skill("missing").is_err());
        });
    }

    #[test]
    fn update_creates_or_overwrites() {
        with_env(|env| {
            let home = temp_home();
            env.set("HOME", home.path());

            let path = update_skill("dynamic", "content v1").expect("update should create");
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "content v1");

            update_skill("dynamic", "content v2").expect("update should overwrite");
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "content v2");
        });
    }
}
