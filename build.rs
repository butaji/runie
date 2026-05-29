use std::fs;
use std::path::Path;

const MAX_FILE_LINES: usize = 500;

fn walk_dir(path: &Path, violations: &mut Vec<(String, usize)>) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            walk_dir(&entry_path, violations);
        } else if entry_path.extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(&entry_path).unwrap();
            let line_count = content.lines().count();
            if line_count > MAX_FILE_LINES {
                violations.push((entry_path.display().to_string(), line_count));
            }
        }
    }
}

fn main() {
    let mut violations = Vec::new();
    walk_dir(Path::new("crates"), &mut violations);

    if !violations.is_empty() {
        eprintln!("\n❌ FILE LINE COUNT VIOLATIONS (max {} lines):\n", MAX_FILE_LINES);
        for (path, count) in &violations {
            eprintln!("  {:6} lines  {}", count, path);
        }
        eprintln!("\nTotal violations: {}\n", violations.len());
        panic!("Build failed: {} files exceed {} lines", violations.len(), MAX_FILE_LINES);
    }

    println!("✓ All Rust source files ≤ {} lines", MAX_FILE_LINES);
}
