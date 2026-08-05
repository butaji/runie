//! `runie-tui-e2e` — load YAML scenarios and run them against the loop.
//!
//! Usage:
//!
//! ```text
//! # Run every fixture under tests/e2e/*.yaml
//! cargo run --bin runie-tui-e2e
//!
//! # Run a specific fixture
//! cargo run --bin runie-tui-e2e -- tests/e2e/hello-streaming.yaml
//! ```
//!
//! YAML fixtures live in `crates/runie-tui/tests/e2e/`. Editing or adding
//! a new `.yaml` file does NOT require rebuilding the binary.

use std::path::PathBuf;
use std::process::ExitCode;

use runie_tui::yaml_runner::{assert_scenario_async, load_scenario, run_scenario};

fn fixture_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR points at the crate root in build scripts and bins.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("e2e");
    p
}

fn discover_fixtures() -> Vec<PathBuf> {
    let dir = fixture_dir();
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("could not read {}: {err}", dir.display());
            return out;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            out.push(path);
        }
    }
    out.sort();
    out
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let fixtures = if args.is_empty() {
        discover_fixtures()
    } else {
        args.into_iter().map(PathBuf::from).collect()
    };

    if fixtures.is_empty() {
        eprintln!("no fixtures found in {}", fixture_dir().display());
        return ExitCode::from(2);
    }

    let mut passed = 0usize;
    let mut failed = 0usize;

    for path in fixtures {
        let scenario = match load_scenario(&path) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("FAIL  {} — load: {err}", path.display());
                failed += 1;
                continue;
            }
        };
        let outcome = match run_scenario(&scenario).await {
            Ok(o) => o,
            Err(err) => {
                eprintln!("FAIL  {} — run: {err}", path.display());
                failed += 1;
                continue;
            }
        };
        match assert_scenario_async(&outcome, &scenario).await {
            Ok(()) => {
                println!("ok    {} ({})", path.display(), scenario.name);
                passed += 1;
            }
            Err(err) => {
                eprintln!("FAIL  {} — {err}", path.display());
                failed += 1;
            }
        }
    }

    println!("\n{passed} passed, {failed} failed");
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}