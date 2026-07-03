//! Skills System — Load SKILL.md files from user and project directories.
//!
//! Supports both flat layout (`name.md`) and nested layout (`name/SKILL.md`).
//! YAML frontmatter is optional and takes precedence over markdown sections.

use camino::Utf8PathBuf;
use std::path::PathBuf;

mod load;
#[cfg(test)]
mod tests;

pub use load::load_from_dir;

/// A loaded skill parsed from a SKILL.md file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub context: String,
    pub user_invocable: bool,
    pub file_path: Utf8PathBuf,
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

/// Load skills from user (`~/.runie/skills/`), project (`./.runie/skills/`),
/// and system (`~/.agents/skills/`) directories.
pub fn load_all() -> Vec<Skill> {
    let mut skills = Vec::new();
    if let Some(home) = dirs::home_dir() {
        skills.extend(load_from_dir(&home.join(".agents").join("skills")));
        skills.extend(load_from_dir(&home.join(".runie").join("skills")));
    }
    skills.extend(load_from_dir(&PathBuf::from(".runie").join("skills")));
    skills
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
