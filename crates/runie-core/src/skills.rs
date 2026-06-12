//! Skills System — Load SKILL.md files from user and project directories.
//!
//! Skills inject context into the system prompt and can be user-invocable.

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

/// Load all SKILL.md files from a directory.
pub fn load_from_dir(dir: &Path) -> Vec<Skill> {
    let mut skills = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return skills,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Some(skill) = parse_skill_md(&path) {
                skills.push(skill);
            }
        }
    }

    skills
}

/// Parse a single SKILL.md file.
fn parse_skill_md(path: &Path) -> Option<Skill> {
    let content = std::fs::read_to_string(path).ok()?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed")
        .to_string();

    let description = extract_section(&content, "Description").unwrap_or_default();
    let context = extract_section(&content, "Context").unwrap_or_default();
    let invocation = extract_section(&content, "Invocation").unwrap_or_default();
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
}
