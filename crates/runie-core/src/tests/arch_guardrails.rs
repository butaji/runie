#![allow(clippy::all)]
//! Architecture guardrails — enforce layer separation (IO | Domain | UI).
//!
//! This test verifies that the domain layer (`crates/runie-core/src/`) does not
//! contain blocking sync IO calls. All file, network, and subprocess operations
//! must live behind actors (IO layer).
//!
//! Test files (*.rs files under tests/ directories) and files that are purely
//! test scaffolding are excluded from this check.

use std::fs;
use std::path::Path;

/// Modules that intentionally own sync IO adapters.
const IO_ADAPTER_MODULES: &[&str] = &["actors/io"];

/// Files in test directories or that are purely test scaffolding.
const TEST_SCAFFOLDING_FILES: &[&str] = &["tests/", "/tests."];

/// Files that intentionally contain sync IO in production.
/// These should be migrated to IO actors over time.
const PRODUCTION_ALLOW_LIST: &[&str] = &[
    // Config loading/writing - owned by ConfigActor but currently in domain
    "config/",
    // Provider-level config resolution (contains sync IO)
    "provider/config.rs",
    // Model config - uses current_dir for path resolution
    "update/agent/model_config.rs",
    // Auth storage - owned by AuthActor but currently in domain
    "auth/",
    // Session persistence - owned by SessionActor but reads use domain paths
    "session/store.rs",
    "session/persistence/",
    // Test scaffolding and support
    "tests/support.rs",
    // Input history persistence - owned by InputActor
    "input_history.rs",
    // Trust settings - owned by TrustActor
    "trust.rs",
    // Atomic file write utility for trust/auth persistence
    "io/atomic_write.rs",
    // Permissions storage
    "actors/permission/",
    // Skills loading
    "skills/",
    // Path utilities used by commands
    "path.rs",
    "path_complete.rs",
    // Hooks for external integrations
    "hooks.rs",
    // Harness skills with file operations
    "harness_skills/",
    // Declarative config loading for skills and commands
    "declarative/",
    // Tool formatting and context
    "tool/",
    // File references
    "file_refs.rs",
    // Prompts loading
    "prompts.rs",
    // Subagent type loading — uses sync IO similar to skills/
    "subagents/",
    // Update modules with tool execution
    "update/tools.rs",
    "update/system.rs",
    "update/dialog/file_pickers.rs",
    "update/dialog/open.rs",
    // Login flow handlers with file IO
    "login_flow/handlers_tests.rs",
    // Actor modules with env/path access
    "actors/completion/",
    "actors/session/actor.rs",
    "actors/session/ractor_session_actor.rs",
    "actors/session/session_handlers.rs",
    // Command handlers with file/dir access
    "commands/dsl/handlers/session/",
    // Leader - coordination layer that manages actor lifecycle
    "actors/leader/",
    // FFF indexer — IO actor owning file search with intentional sync file reads
    "actors/fff_indexer/",
];

/// Patterns that indicate sync IO in production domain code.
const SYNC_IO_PATTERNS: &[(&str, &str)] = &[
    ("std::fs::", "file system operations"),
    ("tokio::fs::", "async file system operations"),
    ("std::env::current_dir", "current working directory access"),
    ("std::env::set_var", "environment mutation"),
    ("std::env::remove_var", "environment mutation"),
    ("std::process::Command", "subprocess execution"),
];

fn is_in_io_adapter_module(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    IO_ADAPTER_MODULES.iter().any(|m| path_str.contains(m))
}

fn is_test_scaffolding(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    TEST_SCAFFOLDING_FILES.iter().any(|f| path_str.contains(f))
}

fn is_on_production_allow_list(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    PRODUCTION_ALLOW_LIST.iter().any(|f| {
        if f.ends_with('/') {
            // Directory pattern: check if path contains the directory
            path_str.contains(*f)
        } else {
            // File pattern: check if path ends with the file
            path_str.ends_with(f)
        }
    })
}

fn should_skip(path: &Path) -> bool {
    is_in_io_adapter_module(path) || is_test_scaffolding(path) || is_on_production_allow_list(path)
}

fn check_file(path: &Path, issues: &mut Vec<String>) {
    if should_skip(path) {
        return;
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for (pattern, description) in SYNC_IO_PATTERNS {
        if content.contains(pattern) {
            issues.push(format!(
                "{}: contains `{}` ({})",
                path.display(),
                pattern,
                description
            ));
        }
    }
}

/// Recursively collect all .rs files in a directory.
fn collect_rust_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_rust_files(&path));
            } else if path.extension().map_or(false, |ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files
}

#[test]
fn no_sync_io_in_domain_core() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut issues = Vec::new();

    for path in collect_rust_files(&crate_root) {
        check_file(&path, &mut issues);
    }

    if !issues.is_empty() {
        let msg = format!(
            "Domain layer contains sync IO ({} violation(s)):\n{}",
            issues.len(),
            issues.join("\n")
        );
        panic!("{}", msg);
    }
}

#[test]
fn no_tokio_fs_in_domain_core() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut issues = Vec::new();

    for path in collect_rust_files(&crate_root) {
        if should_skip(&path) {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if content.contains("tokio::fs::") {
            issues.push(format!("{}: contains `tokio::fs::`", path.display()));
        }
    }

    if !issues.is_empty() {
        let msg = format!(
            "Domain layer contains tokio::fs (should be in IO actors):\n{}",
            issues.join("\n")
        );
        panic!("{}", msg);
    }
}
