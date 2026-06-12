//! `scenario_fasthot` — single-process hot loop for TUI development.
//!
//! Holds the Tui in memory, watches the scenario file, and re-renders +
//! diffs to stdout the instant a save is detected. No per-iteration
//! process startup, no Cargo rebuild between identical renders.
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin scenario_fasthot -- ui/dumps/scenarios/03_chat_simple.jsonl \
//!     --diff ui/dumps/grok/03_chat.txt
//! ```
//!
//! On save the scenario file is re-read and re-played. If the render
//! matches the previous output, nothing is printed (no churn). If it
//! changes, the full new render is printed so the user sees the diff
//! in their terminal.
//!
//! The output is plain text — pipe through `diff -u` or your own
//! diffing tool if you want a unified diff.
//!
//! For source-level changes (`.rs` files), use `bin/runie-rerun`
//! which handles the rebuild step. `scenario_fasthot` is for fast
//! scenario iteration WITHOUT rebuilds.

use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use runie_tui::replay::{apply_scenario, load_scenario, normalize, render_to_text};
use runie_tui::Tui;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: scenario_fasthot <scenario.jsonl> [--diff grok.txt] [--width N] [--height N]");
        return ExitCode::from(2);
    }
    let scenario_path = PathBuf::from(&args[1]);
    let mut diff_path: Option<PathBuf> = None;
    let mut width: u16 = 80;
    let mut height: u16 = 24;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--diff" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--diff requires a path argument");
                    return ExitCode::from(2);
                }
                diff_path = Some(PathBuf::from(&args[i]));
            }
            "--width" => {
                i += 1;
                width = args[i].parse().unwrap_or(80);
            }
            "--height" => {
                i += 1;
                height = args[i].parse().unwrap_or(24);
            }
            _ => {
                eprintln!("unknown arg: {}", args[i]);
                return ExitCode::from(2);
            }
        }
        i += 1;
    }

    eprintln!(
        "scenario_fasthot: watching {} (diff: {})",
        scenario_path.display(),
        diff_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none)".to_string())
    );

    let mut last_render: Option<String> = None;
    let mut last_mtime: Option<std::time::SystemTime> = None;
    let mut tui = Tui::new_headless();

    loop {
        let mtime = std::fs::metadata(&scenario_path)
            .and_then(|m| m.modified())
            .ok();
        if mtime != last_mtime {
            last_mtime = mtime;
            let t0 = Instant::now();
            match run_once(&scenario_path, diff_path.as_deref(), width, height, &mut tui) {
                Ok(text) => {
                    let elapsed_ms = t0.elapsed().as_millis();
                    if last_render.as_deref() != Some(text.as_str()) {
                        last_render = Some(text.clone());
                        let mut out = std::io::stdout().lock();
                        let _ = writeln!(out, "\n─── render {}ms ───", elapsed_ms);
                        let _ = writeln!(out, "{}", text);
                        let _ = out.flush();
                    }
                }
                Err(e) => {
                    eprintln!("[scenario_fasthot] error: {}", e);
                }
            }
        }
        // 10ms is enough to feel real-time (sub-15ms save-to-render
        // total) without burning CPU. With 100 ticks/sec the overhead
        // is negligible on a single core.
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

type ExitCode = std::process::ExitCode;

fn run_once(
    scenario: &std::path::Path,
    diff: Option<&std::path::Path>,
    width: u16,
    height: u16,
    tui: &mut Tui,
) -> Result<String, String> {
    let parsed = load_scenario(scenario)?;
    let w = parsed.width.unwrap_or(width);
    let h = parsed.height.unwrap_or(height);
    apply_scenario(tui, &parsed);
    let rendered = render_to_text(tui, w, h);
    if let Some(diff_path) = diff {
        let expected = std::fs::read_to_string(diff_path).map_err(|e| e.to_string())?;
        let rendered_n = normalize(&rendered);
        let expected_n = normalize(&expected);
        if rendered_n != expected_n {
            print_diff(&expected_n, &rendered_n);
        } else {
            eprintln!("[scenario_fasthot] ✓ match ({} bytes)", rendered.len());
        }
    }
    Ok(rendered)
}

fn print_diff(expected: &str, actual: &str) {
    // Line-by-line diff. We don't use a real LCS — the user only needs
    // to see *what* changed, and a unified diff would need an extra
    // dep.
    let exp: Vec<&str> = expected.lines().collect();
    let act: Vec<&str> = actual.lines().collect();
    let max = exp.len().max(act.len());
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(
        stdout,
        "\n─── diff ({} → {} lines) ───",
        exp.len(),
        act.len()
    );
    for i in 0..max {
        let e = exp.get(i).copied().unwrap_or("");
        let a = act.get(i).copied().unwrap_or("");
        if e == a {
            continue;
        }
        if !e.is_empty() {
            let _ = writeln!(stdout, "-{}", e);
        }
        if !a.is_empty() {
            let _ = writeln!(stdout, "+{}", a);
        }
    }
    let _ = stdout.flush();
}
