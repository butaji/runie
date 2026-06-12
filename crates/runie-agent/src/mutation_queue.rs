//! File mutation queue — serializes edits to avoid conflicts.

use std::collections::VecDeque;
use std::path::PathBuf;

/// A pending file mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum Mutation {
    Write {
        path: PathBuf,
        content: String,
    },
    Edit {
        path: PathBuf,
        old: String,
        new: String,
    },
}

/// Result of applying a single mutation.
#[derive(Debug, Clone, PartialEq)]
pub struct MutationResult {
    pub path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}

/// Queue that serializes file mutations.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileMutationQueue {
    pending: VecDeque<Mutation>,
}

impl FileMutationQueue {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, mutation: Mutation) {
        self.pending.push_back(mutation);
    }

    /// Flush all pending mutations sequentially.
    /// Stops on first failure and returns results so far.
    pub fn flush(&mut self) -> Vec<MutationResult> {
        let mut results = Vec::new();
        while let Some(mutation) = self.pending.pop_front() {
            let result = apply_mutation(&mutation);
            let failed = !result.success;
            results.push(result);
            if failed {
                break;
            }
        }
        results
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

fn apply_mutation(mutation: &Mutation) -> MutationResult {
    match mutation {
        Mutation::Write { path, content } => apply_write(path, content),
        Mutation::Edit { path, old, new } => apply_edit(path, old, new),
    }
}

fn apply_write(path: &std::path::Path, content: &str) -> MutationResult {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return MutationResult {
                    path: path.to_path_buf(),
                    success: false,
                    error: Some(format!("Error creating parent directories: {}", e)),
                };
            }
        }
    }
    match std::fs::write(path, content) {
        Ok(()) => MutationResult {
            path: path.to_path_buf(),
            success: true,
            error: None,
        },
        Err(e) => MutationResult {
            path: path.to_path_buf(),
            success: false,
            error: Some(format!("Error writing {}: {}", path.display(), e)),
        },
    }
}

fn apply_edit(path: &std::path::Path, old: &str, new: &str) -> MutationResult {
    if old.is_empty() {
        return MutationResult {
            path: path.to_path_buf(),
            success: false,
            error: Some("Error: search text cannot be empty".to_string()),
        };
    }
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return MutationResult {
                path: path.to_path_buf(),
                success: false,
                error: Some(format!("Error reading {}: {}", path.display(), e)),
            }
        }
    };
    if let Err(e) = validate_edit(&content, old, path) {
        return e;
    }
    let new_content = content.replacen(old, new, 1);
    write_result(path, &new_content)
}

fn validate_edit(content: &str, old: &str, path: &std::path::Path) -> Result<(), MutationResult> {
    let count = content.matches(old).count();
    if count == 0 {
        return Err(MutationResult {
            path: path.to_path_buf(),
            success: false,
            error: Some(format!(
                "Error: search text not found in {}",
                path.display()
            )),
        });
    }
    if count > 1 {
        return Err(MutationResult {
            path: path.to_path_buf(),
            success: false,
            error: Some(format!(
                "Error: search text appears {} times in {}. Be more specific.",
                count,
                path.display()
            )),
        });
    }
    Ok(())
}

fn write_result(path: &std::path::Path, content: &str) -> MutationResult {
    match std::fs::write(path, content) {
        Ok(()) => MutationResult {
            path: path.to_path_buf(),
            success: true,
            error: None,
        },
        Err(e) => MutationResult {
            path: path.to_path_buf(),
            success: false,
            error: Some(format!("Error writing {}: {}", path.display(), e)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_executes_in_order() {
        let mut queue = FileMutationQueue::new();
        let dir = tempfile::tempdir().unwrap();
        let path1 = dir.path().join("a.txt");
        let path2 = dir.path().join("b.txt");

        queue.enqueue(Mutation::Write {
            path: path1.clone(),
            content: "hello".into(),
        });
        queue.enqueue(Mutation::Write {
            path: path2.clone(),
            content: "world".into(),
        });

        assert_eq!(queue.len(), 2);
        let results = queue.flush();
        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(results[1].success);
        assert!(queue.is_empty());

        assert_eq!(std::fs::read_to_string(&path1).unwrap(), "hello");
        assert_eq!(std::fs::read_to_string(&path2).unwrap(), "world");
    }

    #[test]
    fn failed_mutation_stops() {
        let mut queue = FileMutationQueue::new();
        let dir = tempfile::tempdir().unwrap();
        let path1 = dir.path().join("a.txt");
        let path2 = dir.path().join("b.txt");

        // First write succeeds
        queue.enqueue(Mutation::Write {
            path: path1.clone(),
            content: "first".into(),
        });
        // Edit with nonexistent search text fails
        queue.enqueue(Mutation::Edit {
            path: path1.clone(),
            old: "nonexistent".into(),
            new: "replacement".into(),
        });
        // This should not execute because previous failed
        queue.enqueue(Mutation::Write {
            path: path2.clone(),
            content: "third".into(),
        });

        let results = queue.flush();
        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(!results[1].success);
        assert!(results[1].error.as_ref().unwrap().contains("not found"));
        // Third mutation should still be in queue
        assert_eq!(queue.len(), 1);
        assert!(!path2.exists());
    }

    #[test]
    fn validation_catches_invalid() {
        let mut queue = FileMutationQueue::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "hello world hello").unwrap();

        queue.enqueue(Mutation::Edit {
            path: path.clone(),
            old: "hello".into(),
            new: "hi".into(),
        });

        let results = queue.flush();
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0]
            .error
            .as_ref()
            .unwrap()
            .contains("appears 2 times"));
    }

    #[test]
    fn validation_catches_empty_search() {
        let mut queue = FileMutationQueue::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "content").unwrap();

        queue.enqueue(Mutation::Edit {
            path: path.clone(),
            old: "".into(),
            new: "x".into(),
        });

        let results = queue.flush();
        assert!(!results[0].success);
        assert!(results[0]
            .error
            .as_ref()
            .unwrap()
            .contains("cannot be empty"));
    }

    #[test]
    fn edit_applies_correctly() {
        let mut queue = FileMutationQueue::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "hello world").unwrap();

        queue.enqueue(Mutation::Edit {
            path: path.clone(),
            old: "world".into(),
            new: "universe".into(),
        });

        let results = queue.flush();
        assert!(results[0].success);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello universe");
    }
}
