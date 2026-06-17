//! Progressive skill discovery and triggering.
//!
//! The registry keeps lightweight metadata in memory and loads the full
//! instruction content only when a skill is triggered.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::skills::{self, Skill};

/// Lightweight skill metadata kept in memory for matching and listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub file_path: PathBuf,
}

/// Registry of discovered skills with lazy instruction loading.
#[derive(Clone, Debug, Default)]
pub struct SkillRegistry {
    skills: Vec<SkillSummary>,
}

impl SkillRegistry {
    /// Discover skills from all configured paths.
    pub fn load_all() -> Self {
        let mut by_name: HashMap<String, SkillSummary> = HashMap::new();
        for path in discovery_paths() {
            for summary in load_summaries_from_dir(&path) {
                by_name.insert(summary.name.clone(), summary);
            }
        }
        Self {
            skills: by_name.into_values().collect(),
        }
    }

    /// List all discovered skill summaries.
    pub fn list_skills(&self) -> &[SkillSummary] {
        &self.skills
    }

    /// Find a skill by exact name.
    pub fn find_skill(&self, name: &str) -> Option<&SkillSummary> {
        self.skills.iter().find(|s| s.name == name)
    }

    /// Trigger the best-matching skill for a user intent.
    ///
    /// Matching prefers exact name matches, then descriptions that contain the
    /// intent substring. The full skill (including instructions) is loaded from
    /// disk for the match.
    pub fn trigger_skill(&self, intent: &str) -> Option<Skill> {
        let intent_lower = intent.to_lowercase();
        let mut best: Option<&SkillSummary> = None;
        let mut best_score = 0;

        for summary in &self.skills {
            let name_lower = summary.name.to_lowercase();
            let desc_lower = summary.description.to_lowercase();
            let score = if name_lower == intent_lower {
                3
            } else if name_lower.contains(&intent_lower) {
                2
            } else if desc_lower.contains(&intent_lower) {
                1
            } else {
                0
            };
            if score > best_score {
                best_score = score;
                best = Some(summary);
            }
        }

        best.and_then(|s| skills::parse_skill_md(&s.file_path))
    }
}

/// All skill discovery paths in increasing priority order. Later paths override
/// earlier paths for skills with the same name.
fn discovery_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".agents").join("skills"));
        paths.push(home.join(".runie").join("skills"));
    }
    paths.push(PathBuf::from(".runie").join("skills"));
    paths
}

/// Load skill summaries from a directory, preferring `name/SKILL.md` over
/// `name.md` for the same skill name.
fn load_summaries_from_dir(dir: &Path) -> Vec<SkillSummary> {
    let mut by_name = load_subdir_summaries(dir);
    load_flat_summaries(dir, &mut by_name);
    by_name.into_values().collect()
}

fn load_subdir_summaries(dir: &Path) -> HashMap<String, SkillSummary> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return HashMap::new(),
    };

    let mut by_name = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        if let Some(summary) = parse_skill_summary(&skill_md) {
            by_name.insert(summary.name.clone(), summary);
        }
    }
    by_name
}

fn load_flat_summaries(dir: &Path, by_name: &mut HashMap<String, SkillSummary>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        if by_name.contains_key(stem) {
            continue;
        }
        if let Some(summary) = parse_skill_summary(&path) {
            by_name.insert(summary.name.clone(), summary);
        }
    }
}

/// Parse only the metadata from a SKILL.md file without loading instructions.
fn parse_skill_summary(path: &Path) -> Option<SkillSummary> {
    let content = std::fs::read_to_string(path).ok()?;
    let frontmatter = skills::extract_frontmatter(&content);

    let name = frontmatter
        .get("name")
        .cloned()
        .or_else(|| derive_name(path))?;
    let description = frontmatter
        .get("description")
        .cloned()
        .or_else(|| skills::extract_section(&content, "Description"))?;

    Some(SkillSummary {
        name,
        description,
        file_path: path.to_owned(),
    })
}

fn derive_name(path: &Path) -> Option<String> {
    if path.file_name().and_then(|s| s.to_str()) == Some("SKILL.md") {
        path.parent()?.file_name()?.to_str().map(|s| s.to_string())
    } else {
        path.file_stem()?.to_str().map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_skill(dir: &Path, name: &str, description: &str, context: &str) {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        write!(
            file,
            "---\nname: {}\ndescription: {}\n---\n\n## Context\n\n{}\n",
            name, description, context
        )
        .unwrap();
    }

    #[test]
    fn skill_discovery_finds_skills_in_priority_paths() {
        let agents_dir = tempdir().unwrap();
        let user_dir = tempdir().unwrap();
        let project_dir = tempdir().unwrap();

        write_skill(agents_dir.path(), "shared", "agents description", "agents context");
        write_skill(user_dir.path(), "shared", "user description", "user context");
        write_skill(project_dir.path(), "shared", "project description", "project context");

        let mut registry = SkillRegistry::default();
        let mut by_name: HashMap<String, SkillSummary> = HashMap::new();
        for dir in [agents_dir.path(), user_dir.path(), project_dir.path()] {
            for summary in load_summaries_from_dir(dir) {
                by_name.insert(summary.name.clone(), summary);
            }
        }
        registry.skills = by_name.into_values().collect();

        let shared = registry.find_skill("shared").expect("shared skill");
        assert_eq!(shared.description, "project description");
    }

    #[test]
    fn skill_matching_selects_best_match() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "rust", "Rust best practices", "Use clippy.");
        write_skill(dir.path(), "python", "Python best practices", "Use black.");

        let registry = SkillRegistry {
            skills: load_summaries_from_dir(dir.path()),
        };

        let triggered = registry.trigger_skill("rust").expect("triggered rust");
        assert_eq!(triggered.name, "rust");
        assert_eq!(triggered.context, "Use clippy.");
    }

    #[test]
    fn progressive_loading_only_loads_instructions_on_trigger() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "rust", "Rust best practices", "Use clippy always.");

        let registry = SkillRegistry {
            skills: load_summaries_from_dir(dir.path()),
        };

        let summary = registry.find_skill("rust").unwrap();
        assert!(!summary.description.contains("clippy"));

        let skill = registry.trigger_skill("rust").unwrap();
        assert!(skill.context.contains("clippy"));
    }
}
