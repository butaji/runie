//! Incremental Rust-native development runner for the TUI.
//!
//! This intentionally keeps the application process attached to the caller's
//! terminal. `cargo run` reuses Cargo's incremental compilation cache, while
//! this small supervisor only detects source changes and restarts the child.

use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, SystemTime};

const POLL_INTERVAL: Duration = Duration::from_millis(250);

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("runie workspace root")
        .to_path_buf()
}

fn source_fingerprint(root: &Path) -> u64 {
    let mut entries = Vec::new();
    for directory in ["crates", "Cargo.toml", "Cargo.lock"] {
        let path = root.join(directory);
        collect_files(&path, &mut entries);
    }
    entries.sort();

    let mut hasher = DefaultHasher::new();
    for path in entries {
        path.hash(&mut hasher);
        if let Ok(metadata) = fs::metadata(&path) {
            metadata.len().hash(&mut hasher);
            metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn collect_files(path: &Path, output: &mut Vec<PathBuf>) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.is_file() {
        if path.extension().is_some_and(|extension| {
            matches!(extension.to_str(), Some("rs" | "toml" | "yaml" | "yml"))
        }) {
            output.push(path.to_path_buf());
        }
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        collect_files(&entry.path(), output);
    }
}

fn start_tui(args: &[String]) -> std::io::Result<Child> {
    Command::new("cargo")
        .args(["run", "-p", "runie-tui", "--bin", "runie", "--"])
        .args(args)
        .current_dir(workspace_root())
        .spawn()
}

fn main() -> std::io::Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let root = workspace_root();
    let mut fingerprint = source_fingerprint(&root);

    loop {
        let mut child = start_tui(&args)?;
        loop {
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            thread::sleep(POLL_INTERVAL);
            let current = source_fingerprint(&root);
            if current != fingerprint {
                fingerprint = current;
                eprintln!("\nsource changed; restarting TUI with incremental Cargo build...\n");
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
        }
    }
}
