use std::path::{Path, PathBuf};

/// Loads AGENTS.md context files from standard locations.
/// Mirrors pi's context loading behavior.
pub struct ContextLoader;

impl ContextLoader {
    /// Load all AGENTS.md context files from standard locations.
    /// Order: global (~/.runie/agent/AGENTS.md), then project hierarchy.
    pub fn load() -> Vec<ContextFile> {
        let mut files = Vec::new();

        // 1. Load from ~/.runie/agent/AGENTS.md (global user rules)
        if let Some(home) = dirs::home_dir() {
            let global = home.join(".runie/agent/AGENTS.md");
            if global.exists() {
                files.push(ContextFile::load(&global));
            }
        }

        // 2. Load from parent directories up to git root (project rules)
        if let Ok(cwd) = std::env::current_dir() {
            let git_root = find_git_root(&cwd);
            let mut current = cwd.clone();

            loop {
                // Check for AGENTS.md
                let agents_md = current.join("AGENTS.md");
                if agents_md.exists() {
                    files.push(ContextFile::load(&agents_md));
                }

                // Check for CLAUDE.md (alternative naming)
                let claude_md = current.join("CLAUDE.md");
                if claude_md.exists() {
                    files.push(ContextFile::load(&claude_md));
                }

                // Stop if we reached git root
                if Some(&current) == git_root.as_ref() {
                    break;
                }

                // Move to parent directory
                if !current.pop() {
                    break;
                }
            }
        }

        files
    }

    /// Get list of loaded context file paths as display strings.
    pub fn loaded_paths() -> Vec<String> {
        Self::load()
            .iter()
            .map(|f| f.path.to_string_lossy().to_string())
            .collect()
    }

    /// Check if .runie/SYSTEM.md exists in cwd for complete override.
    pub fn system_override() -> Option<String> {
        if let Ok(cwd) = std::env::current_dir() {
            let override_path = cwd.join(".runie/SYSTEM.md");
            if override_path.exists() {
                return std::fs::read_to_string(&override_path).ok();
            }
        }
        None
    }
}

/// A loaded context file with its path and content.
#[derive(Debug, Clone)]
pub struct ContextFile {
    pub path: PathBuf,
    pub content: String,
}

impl ContextFile {
    /// Load a context file from disk.
    pub fn load(path: &Path) -> Self {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        Self {
            path: path.to_path_buf(),
            content,
        }
    }
}

/// Find the git repository root starting from a directory.
fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Build a system prompt from loaded context files.
pub fn build_system_prompt(context_files: &[ContextFile]) -> String {
    if context_files.is_empty() {
        return String::from("You are a helpful coding assistant.");
    }

    let mut prompt = String::new();
    prompt.push_str("You are a helpful coding assistant.\n");

    for file in context_files {
        prompt.push_str("\n# Context from ");
        prompt.push_str(&file.path.to_string_lossy());
        prompt.push_str("\n\n");
        prompt.push_str(&file.content);
        prompt.push('\n');
    }

    prompt
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_file_load_nonexistent() {
        let file = ContextFile::load(Path::new("/nonexistent/file.md"));
        assert!(file.content.is_empty());
    }

    #[test]
    fn test_find_git_root() {
        let current = std::env::current_dir().unwrap();
        let root = find_git_root(&current);
        // Should find git root in a git repo
        assert!(root.is_some());
        assert!(root.unwrap().join(".git").exists());
    }

    #[test]
    fn test_build_system_prompt_empty() {
        let files: Vec<ContextFile> = vec![];
        let prompt = build_system_prompt(&files);
        assert_eq!(prompt, "You are a helpful coding assistant.");
    }

    #[test]
    fn test_build_system_prompt_with_files() {
        let files = vec![
            ContextFile {
                path: PathBuf::from("/home/user/.runie/agent/AGENTS.md"),
                content: "Global rules".to_string(),
            },
            ContextFile {
                path: PathBuf::from("/project/AGENTS.md"),
                content: "Project rules".to_string(),
            },
        ];
        let prompt = build_system_prompt(&files);
        assert!(prompt.contains("Global rules"));
        assert!(prompt.contains("Project rules"));
        assert!(prompt.contains("# Context from"));
    }
}
