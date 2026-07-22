//! Skills System — Load SKILL.md files from user and project directories.
//!
//! Supports both flat layout (`name.md`) and nested layout (`name/SKILL.md`).
//! YAML frontmatter is optional and takes precedence over markdown sections.
//!
//! # Scope Priority (highest to lowest)
//! 1. Local: `.runie/skills/` in cwd
//! 2. Repo: `.runie/skills/` in repo root (via git discovery)
//! 3. User: `~/.agents/skills/`, `~/.runie/skills/`
//! 4. Config: Paths from `config.toml` skills.paths
//! 5. Server: Server-synced skills (injected)
//! 6. Bundled: Built-in skills (lowest priority)

use camino::Utf8PathBuf;
use std::path::{Path, PathBuf};

mod load;
#[cfg(test)]
mod tests;

pub use load::load_from_dir;

/// Skill discovery scope (priority order: Local highest, Bundled lowest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillScope {
    /// `.runie/skills/` in current working directory.
    Local = 0,
    /// `.runie/skills/` in git repo root.
    Repo = 1,
    /// `~/.agents/skills/`, `~/.runie/skills/`
    User = 2,
    /// Paths from `config.toml` skills.paths.
    Config = 3,
    /// Server-synced skills (injected at runtime).
    Server = 4,
    /// Built-in/bundled skills (lowest priority).
    Bundled = 5,
}

impl std::fmt::Display for SkillScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillScope::Local => write!(f, "Local"),
            SkillScope::Repo => write!(f, "Repo"),
            SkillScope::User => write!(f, "User"),
            SkillScope::Config => write!(f, "Config"),
            SkillScope::Server => write!(f, "Server"),
            SkillScope::Bundled => write!(f, "Bundled"),
        }
    }
}

/// A loaded skill parsed from a SKILL.md file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub context: String,
    pub user_invocable: bool,
    pub file_path: Utf8PathBuf,
    pub scope: SkillScope,
    pub enabled: bool,
    pub plugin_name: Option<String>,
    pub ignore_paths: Vec<String>,
}

impl Skill {
    /// Build a one-line summary for listing.
    pub fn summary(&self) -> String {
        let invocable = if self.user_invocable { " (invocable)" } else { "" };
        let scope_tag = format!(" [{}]", self.scope);
        let qualified = self.qualified_name();
        format!("{}{}{} — {}", qualified, scope_tag, invocable, self.description)
    }

    /// Returns the qualified name for plugin-provided skills.
    /// Format: `plugin_name/skill_name` for plugins, bare `skill_name` otherwise.
    pub fn qualified_name(&self) -> String {
        match &self.plugin_name {
            Some(p) => format!("{}/{}", p, self.name),
            None => self.name.clone(),
        }
    }
}

/// Load skills from all scopes, sorted by priority (Local highest, Bundled lowest).
///
/// `$HOME` is honored first so isolated test environments (and users who set
/// it explicitly) get the expected skills directory.
pub fn load_all() -> Vec<Skill> {
    let mut skills = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from).or_else(dirs::home_dir);

    // Local scope: .runie/skills/ in cwd
    if let Ok(cwd) = std::env::current_dir() {
        skills.extend(load_from_dir_scoped(&cwd.join(".runie").join("skills"), SkillScope::Local));
    }

    // Repo scope: .runie/skills/ in git root
    #[cfg(feature = "git")]
    if let Some(_home) = &home {
        // Try git repo root
        if let Ok(repo) = git2::Repository::discover(".") {
            if let Some(root) = repo.workdir() {
                let repo_root_skills = root.join(".runie").join("skills");
                if repo_root_skills != cwd().map(|c| c.join(".runie").join("skills")).unwrap_or_default() {
                    skills.extend(load_from_dir_scoped(&repo_root_skills, SkillScope::Repo));
                }
            }
        }
    }

    // User scope: ~/.agents/skills/, ~/.runie/skills/
    // Bundled scope: built-in skills
    if let Some(home) = home {
        skills.extend(load_from_dir_scoped(&home.join(".agents").join("skills"), SkillScope::User));
        skills.extend(load_from_dir_scoped(&home.join(".runie").join("skills"), SkillScope::User));
        skills.extend(load_from_dir_scoped(&home.join(".runie").join("bundled"), SkillScope::Bundled));
    }

    // Filter disabled skills (reads from config if available)
    filter_disabled_skills(&mut skills);

    // Deduplicate by name, keeping highest priority (first occurrence wins)
    deduplicate_skills(skills)
}

#[allow(dead_code)]
fn cwd() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

/// Load skills from a directory with a specific scope.
fn load_from_dir_scoped(dir: &Path, scope: SkillScope) -> Vec<Skill> {
    load_from_dir(dir).into_iter().map(|mut s| {
        s.scope = scope;
        s.enabled = true;
        s
    }).collect()
}

/// Filter skills based on disabled list from config.
fn filter_disabled_skills(skills: &mut [Skill]) {
    // Load disabled skills from config
    let config = runie_core::config::Config::load(None);
    let disabled: std::collections::HashSet<_> = config.skills.disabled.iter().cloned().collect();
    for skill in skills.iter_mut() {
        skill.enabled = !disabled.contains(&skill.name) && !disabled.contains(&skill.qualified_name());
    }
}

/// Deduplicate skills by name, keeping the highest priority (lowest scope value).
fn deduplicate_skills(mut skills: Vec<Skill>) -> Vec<Skill> {
    skills.sort_by_key(|s| s.scope);
    let mut seen = std::collections::HashSet::new();
    skills.into_iter().filter(|s| seen.insert(s.name.clone())).collect()
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
