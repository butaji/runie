//! `scenario_replay` — replay a scenario into a Tui, render the result,
//! optionally diff against a Grok reference dump.
//!
//! # REPL-like loop for TUI parity work
//!
//! Edit a render function → re-run this binary → see the new render in
//! <500ms because it reuses the cached `scenario_replay` binary.
//!
//! ```bash
//! # Render a scenario to stdout
//! cargo run -p runie-tui --bin scenario_replay --release -- \
//!     ui/dumps/scenarios/01_welcome.jsonl
//!
//! # Compare against a Grok reference dump (exit 0=match, 1=different)
//! cargo run -p runie-tui --bin scenario_replay --release -- \
//!     ui/dumps/scenarios/01_welcome.jsonl --diff ui/dumps/grok/01_welcome.txt
//!
//! # Wireframe overlay
//! RUNIE_WIREFRAME=all cargo run -p runie-tui --bin scenario_replay --release -- \
//!     ui/dumps/scenarios/01_welcome.jsonl
//! ```
//!
//! # Scenario file format
//!
//! JSON Lines, one event per line. The first non-blank/non-comment line
//! MAY be a `__meta__` object with `width` and `height` (terminal size).
//! Lines starting with `#` are comments. Blank lines are ignored.
//!
//! Each subsequent line is one of:
//!
//! 1. **`AgentEvent`** (tag = "type"): the agent stream, same shape as
//!    `runie_agent::events::AgentEvent`. Drives messages, thinking,
//!    tool calls, token usage, etc.
//! 2. **`ui`** (tag = "ui"): sets UI state directly. Keys:
//!      - `home_visible: bool`  — show/hide the home screen
//!      - `home_selected: u8`   — index of selected home menu item
//!      - `show_sessions: bool` — show session list instead of welcome
//!      - `slash_open: bool`    — open/close slash menu
//!      - `slash_query: "..."`  — filter the slash menu
//!      - `input: "..."`        — replace text in input box
//!      - `input_insert: "..."` — append text in input box
//!      - `mode: "home"|"chat"|"onboarding"|"command_palette"`
//!      - `git: { repo, branch, path }` — set top bar git info
//!
//! The first call to `ui` switches to the specified mode; subsequent
//! `ui` events are idempotent (most are setters).

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use runie_agent::events::AgentEvent;
use runie_tui::tui::Tui;
use runie_tui::tui::TuiMode;
use runie_tui::components::SlashMenu;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: scenario_replay <scenario.jsonl> [out.txt] [--diff <ref.txt>] [--width N] [--height N]");
        return ExitCode::from(2);
    }

    let mut scenario_path: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;
    let mut diff_path: Option<PathBuf> = None;
    let mut width: u16 = 80;
    let mut height: u16 = 24;
    let mut i = 1;
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
                if i >= args.len() {
                    eprintln!("--width requires a number");
                    return ExitCode::from(2);
                }
                width = args[i].parse().expect("--width must be a number");
            }
            "--height" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("--height requires a number");
                    return ExitCode::from(2);
                }
                height = args[i].parse().expect("--height must be a number");
            }
            "--help" | "-h" => {
                eprintln!("see usage in source");
                return ExitCode::from(0);
            }
            other => {
                if scenario_path.is_none() { scenario_path = Some(PathBuf::from(other)); }
                else if out_path.is_none() { out_path = Some(PathBuf::from(other)); }
                else { eprintln!("unexpected arg: {other}"); return ExitCode::from(2); }
            }
        }
        i += 1;
    }
    let scenario_path = match scenario_path {
        Some(p) => p,
        None => { eprintln!("error: no scenario file"); return ExitCode::from(2); }
    };

    let mut raw = String::new();
    if let Err(e) = std::fs::File::open(&scenario_path).and_then(|mut f| f.read_to_string(&mut raw))
    {
        eprintln!("error reading {}: {e}", scenario_path.display());
        return ExitCode::from(1);
    }
    let parsed = match parse_scenario(&raw) {
        Ok(p) => p,
        Err(e) => { eprintln!("error parsing {}: {e}", scenario_path.display()); return ExitCode::from(1); }
    };
    if let Some(w) = parsed.width { width = w; }
    if let Some(h) = parsed.height { height = h; }
    eprintln!(
        "[scenario_replay] {} actions, terminal {}x{}",
        parsed.actions.len(), width, height
    );

    let mut tui = Tui::new_headless();

    // Apply UI ops + agent events in their original file order. The
    // scenario file may interleave them — e.g. set_context_window op
    // at the start, then agent events, then set_thought_duration op
    // at the end after the assistant message has been created.
    for action in &parsed.actions {
        // apply_ui_op returns terminal commands (we drop them, headless)
        // on_agent_event returns ()
        if let ScenarioAction::UiOp(op) = action {
            let _ = apply_ui_op(&mut tui, op);
        } else if let ScenarioAction::Event(ev) = action {
            tui.on_agent_event(ev.clone());
        }
    }
    let rendered = render_to_text(&mut tui, width, height);

    if let Some(diff_path) = diff_path {
        let reference = match std::fs::read_to_string(&diff_path) {
            Ok(s) => s,
            Err(e) => { eprintln!("error: {e}"); return ExitCode::from(1); }
        };
        let rendered_trim = normalize(&rendered);
        let ref_trim = normalize(&reference);
        if rendered_trim == ref_trim {
            eprintln!("[scenario_replay] ✓ match: {} bytes", rendered_trim.len());
            return ExitCode::from(0);
        }
        eprintln!("[scenario_replay] ✗ diff: runie vs {}", diff_path.display());
        print_diff(&ref_trim, &rendered_trim);
        return ExitCode::from(1);
    }

    match out_path {
        Some(p) => {
            std::fs::write(&p, &rendered).unwrap_or_else(|e| { eprintln!("error: {e}"); });
            eprintln!("[scenario_replay] wrote {} ({} bytes)", p.display(), rendered.len());
        }
        None => {
            let stdout = io::stdout();
            let mut h = stdout.lock();
            h.write_all(rendered.as_bytes()).expect("stdout write");
        }
    }
    ExitCode::from(0)
}

#[derive(Debug)]
struct ParsedScenario {
    width: Option<u16>,
    height: Option<u16>,
    actions: Vec<ScenarioAction>,
}

#[derive(Debug, Clone)]
enum ScenarioAction {
    UiOp(UiOp),
    Event(AgentEvent),
}

#[derive(Debug, Clone)]
enum UiOp {
    SetMode(TuiMode),
    SetHomeVisible(bool),
    SetHomeSelected(usize),
    SetShowSessions(bool),
    SetSlashOpen(bool),
    SetSlashQuery(String),
    SetInput(String),
    AppendInput(String),
    SetGit {
        repo: String,
        branch: String,
        path: String,
    },
    SetContextWindow(usize),
    SetSessionTokens { total: usize },
    SetThoughtDuration(f32),
    SetTurnComplete(f32),
    SetToolResult { name: String, result: String, is_error: bool },
}

fn parse_scenario(raw: &str) -> Result<ParsedScenario, String> {
    let mut out = ParsedScenario {
        width: None,
        height: None,
        actions: Vec::new(),
    };
    for (line_no, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| format!("line {}: {e}", line_no + 1))?;
        if v.get("__meta__").and_then(|x| x.as_bool()).unwrap_or(false) {
            out.width = v.get("width").and_then(|x| x.as_u64()).map(|n| n as u16);
            out.height = v.get("height").and_then(|x| x.as_u64()).map(|n| n as u16);
            continue;
        }
        if v.get("ui").and_then(|x| x.as_bool()).unwrap_or(false) {
            let op = parse_ui_op(&v).map_err(|e| format!("line {} (ui): {e}", line_no + 1))?;
            out.actions.push(ScenarioAction::UiOp(op));
            continue;
        }
        let event: AgentEvent = serde_json::from_value(v)
            .map_err(|e| format!("line {} (agent event): {e}", line_no + 1))?;
        out.actions.push(ScenarioAction::Event(event));
    }
    Ok(out)
}

fn parse_ui_op(v: &serde_json::Value) -> Result<UiOp, String> {
    let kind = v.get("kind").and_then(|x| x.as_str()).ok_or("missing 'kind'")?;
    let get_str = |k: &str| v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string());
    let get_bool = |k: &str| v.get(k).and_then(|x| x.as_bool());
    let get_u64 = |k: &str| v.get(k).and_then(|x| x.as_u64());
    match kind {
        "mode" => {
            let s = get_str("value").ok_or("missing 'value'")?;
            match s.as_str() {
                "home" => Ok(UiOp::SetMode(TuiMode::HomeScreen)),
                "chat" => Ok(UiOp::SetMode(TuiMode::Chat)),
                "onboarding" => Ok(UiOp::SetMode(TuiMode::Onboarding)),
                "command_palette" => Ok(UiOp::SetMode(TuiMode::CommandPalette)),
                other => Err(format!("unknown mode '{other}'")),
            }
        }
        "home_visible" => Ok(UiOp::SetHomeVisible(get_bool("value").ok_or("missing 'value'")?)),
        "home_selected" => Ok(UiOp::SetHomeSelected(get_u64("value").ok_or("missing 'value'")? as usize)),
        "show_sessions" => Ok(UiOp::SetShowSessions(get_bool("value").ok_or("missing 'value'")?)),
        "slash_open" => Ok(UiOp::SetSlashOpen(get_bool("value").ok_or("missing 'value'")?)),
        "slash_query" => Ok(UiOp::SetSlashQuery(get_str("value").ok_or("missing 'value'")?)),
        "input" => Ok(UiOp::SetInput(get_str("value").ok_or("missing 'value'")?)),
        "input_insert" => Ok(UiOp::AppendInput(get_str("value").ok_or("missing 'value'")?)),
        "git" => {
            let g = v.get("value").ok_or("missing 'value'")?;
            Ok(UiOp::SetGit {
                repo: g.get("repo").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                branch: g.get("branch").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                path: g.get("path").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            })
        }
        "context_window" => Ok(UiOp::SetContextWindow(
            get_u64("value").ok_or("missing 'value'")? as usize,
        )),
        "session_tokens" => {
            let g = v.get("value").ok_or("missing 'value'")?;
            Ok(UiOp::SetSessionTokens {
                total: g.get("total").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
            })
        }
        "thought_duration" => {
            let secs = v.get("value").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
            Ok(UiOp::SetThoughtDuration(secs))
        }
        "turn_complete" => {
            let secs = v.get("value").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
            Ok(UiOp::SetTurnComplete(secs))
        }
        "tool_result" => {
            let g = v.get("value").ok_or("missing 'value'")?;
            let name = g
                .get("name")
                .and_then(|x| x.as_str())
                .ok_or("missing name")?
                .to_string();
            let result = g
                .get("result")
                .and_then(|x| x.as_str())
                .ok_or("missing result")?
                .to_string();
            let is_error = g.get("is_error").and_then(|x| x.as_bool()).unwrap_or(false);
            Ok(UiOp::SetToolResult { name, result, is_error })
        }
        other => Err(format!("unknown ui kind '{other}'")),
    }
}

fn apply_ui_op(tui: &mut Tui, op: &UiOp) {
    let state = &mut tui.state;
    match op {
        UiOp::SetMode(m) => state.mode = m.clone(),
        UiOp::SetHomeVisible(v) => {
            if *v { state.home_screen.show(); } else { state.home_screen.hide(); }
        }
        UiOp::SetHomeSelected(n) => state.home_screen.selected = *n,
        UiOp::SetShowSessions(v) => state.home_screen.show_sessions = *v,
        UiOp::SetSlashOpen(open) => {
            if *open {
                state.slash_menu = SlashMenu::new();
                state.slash_menu.open("");
            } else {
                state.slash_menu.close();
            }
        }
        UiOp::SetSlashQuery(q) => {
            state.slash_menu.open(q);
        }
        UiOp::SetInput(s) => {
            state.textarea = ratatui_textarea::TextArea::default();
            state.textarea.insert_str(s);
        }
        UiOp::AppendInput(s) => {
            state.textarea.insert_str(s);
        }
        UiOp::SetGit { repo, branch, path } => {
            state.context.repo = repo.clone();
            state.context.branch = branch.clone();
            state.context.path = path.clone();
            // Top bar also has its own copy
            state.top_bar.repo = repo.clone();
            state.top_bar.branch = branch.clone();
            state.top_bar.path = path.clone();
        }
        UiOp::SetContextWindow(n) => {
            state.top_bar.context_window = Some(*n);
            // Top bar's `estimated_tokens` is what the gauge reads for "used"
            state.top_bar.estimated_tokens = Some(state.session_token_usage.total_tokens);
        }
        UiOp::SetSessionTokens { total } => {
            state.session_token_usage.total_tokens = *total;
            state.top_bar.estimated_tokens = Some(*total);
        }
        UiOp::SetThoughtDuration(secs) => {
            // Set the last assistant's thought_duration so the
            // "◆ Thought for X.Xs" header renders.
            if let Some(runie_tui::components::MessageItem::Assistant {
                thought_duration,
                ..
            }) = state.messages.last_mut()
            {
                *thought_duration = Some(*secs);
            }
        }
        UiOp::SetTurnComplete(secs) => {
            // Set the last assistant's turn_duration so the
            // "Turn completed in X.Xs." line renders.
            if let Some(runie_tui::components::MessageItem::Assistant {
                turn_duration,
                ..
            }) = state.messages.last_mut()
            {
                *turn_duration = Some(*secs);
            }
        }
        UiOp::SetToolResult { name, result, is_error } => {
            // Find the last tool item (ToolCall or ToolRunning) matching
            // `name` and set its result. ToolRunning gets converted to
            // ToolCall so the renderer shows the result instead of the
            // spinner.
            for item in state.messages.iter_mut().rev() {
                match item {
                    runie_tui::components::MessageItem::ToolCall {
                        name: tname,
                        result: tres,
                        is_error: terr,
                        ..
                    } if tname == name => {
                        *tres = Some(result.clone());
                        *terr = *is_error;
                        break;
                    }
                    runie_tui::components::MessageItem::ToolRunning {
                        name: tname,
                        args,
                        ..
                    } if tname == name => {
                        // Convert in-place to a completed ToolCall
                        let tname = tname.clone();
                        let args = args.clone();
                        *item = runie_tui::components::MessageItem::ToolCall {
                            name: tname,
                            args,
                            result: Some(result.clone()),
                            is_error: *is_error,
                        };
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn render_to_text(tui: &mut Tui, width: u16, height: u16) -> String {
    if width < 2 || height < 2 {
        eprintln!("error: terminal dimensions must be >= 2x2 (got {width}x{height})");
        std::process::exit(2);
    }
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("TestBackend init");
    let _ = terminal.draw(|frame| {
        let area = frame.area();
        // Render directly into the frame's buffer -- no clone needed.
        tui.render_to_buffer(frame.buffer_mut(), area);
    });
    let buf = terminal.backend().buffer().clone();
    let mut s = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = buf.cell((x, y)).unwrap();
            s.push_str(cell.symbol());
        }
        if y + 1 < buf.area.height { s.push('\n'); }
    }
    s
}

fn normalize(s: &str) -> String {
    s.lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n")
}

fn print_diff(a: &str, b: &str) {
    let a_lines: Vec<&str> = a.lines().collect();
    let b_lines: Vec<&str> = b.lines().collect();
    let n = a_lines.len();
    let m = b_lines.len();
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            lcs[i + 1][j + 1] = if a_lines[i] == b_lines[j] {
                lcs[i][j] + 1
            } else {
                lcs[i][j + 1].max(lcs[i + 1][j])
            };
        }
    }
    let mut i = n;
    let mut j = m;
    let mut out: Vec<(char, &str)> = Vec::new();
    while i > 0 && j > 0 {
        if a_lines[i - 1] == b_lines[j - 1] {
            out.push((' ', a_lines[i - 1]));
            i -= 1; j -= 1;
        } else if lcs[i - 1][j] >= lcs[i][j - 1] {
            out.push(('-', a_lines[i - 1])); i -= 1;
        } else {
            out.push(('+', b_lines[j - 1])); j -= 1;
        }
    }
    while i > 0 { out.push(('-', a_lines[i - 1])); i -= 1; }
    while j > 0 { out.push(('+', b_lines[j - 1])); j -= 1; }
    out.reverse();
    for (sign, line) in &out {
        println!("{sign} {line}");
    }
}
