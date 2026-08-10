#![allow(
    clippy::manual_repeat_n,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    reason = "fixture comparisons intentionally keep each recorded frame assertion together"
)]

use std::path::PathBuf;

use serde_json::Value;

use runie_tui::yaml_runner::{load_scenario, render_visual_buffer};

#[tokio::test]
async fn hey_yaml_replays_across_capture_matrix_sizes() {
    let scenario = load_scenario(&fixture("visual-hey.yaml")).expect("Hey fixture");
    let sizes = [(62, 32), (80, 24), (100, 30), (120, 36)];
    for (cols, rows) in sizes {
        let mut visual = scenario
            .assertions
            .visual
            .clone()
            .expect("Hey visual assertions");
        visual.cols = cols;
        visual.rows = rows;
        let buffer = render_visual_buffer(&scenario, &visual)
            .await
            .unwrap_or_else(|error| panic!("{cols}x{rows}: {error}"));
        assert_eq!(buffer.area().width, cols);
        assert_eq!(buffer.area().height, rows);
        assert!(
            buffer.content().iter().any(|cell| cell.symbol() == "❯"),
            "{cols}x{rows}: missing user prompt cursor"
        );
    }
}

#[tokio::test]
async fn unsupported_slash_command_replays_across_capture_matrix_sizes() {
    let scenario = load_scenario(&fixture("visual-slash-unsupported.yaml"))
        .expect("unsupported slash-command fixture");
    for (cols, rows) in [(62, 32), (80, 24), (100, 30), (120, 36)] {
        let mut visual = scenario
            .assertions
            .visual
            .clone()
            .expect("unsupported visual assertions");
        visual.cols = cols;
        visual.rows = rows;
        let buffer = render_visual_buffer(&scenario, &visual)
            .await
            .unwrap_or_else(|error| panic!("{cols}x{rows}: {error}"));
        let screen = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(
            screen.contains("not supported by Runie"),
            "{cols}x{rows}: missing unsupported-command diagnostic"
        );
    }
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/e2e")
        .join(name)
}

struct CastDump {
    name: &'static str,
    cols: u16,
    rows: u16,
    frames: usize,
    final_screen: String,
}

fn replay_cast(path: &str, name: &'static str) -> CastDump {
    let cast = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../artifacts")
            .join(path),
    )
    .unwrap_or_else(|error| panic!("read asciinema dump {path}: {error}"));
    let mut lines = cast.lines();
    let header: Value = serde_json::from_str(lines.next().expect("cast header"))
        .unwrap_or_else(|error| panic!("parse asciinema header {path}: {error}"));
    let term = header.get("term").expect("cast terminal metadata");
    let cols = term["cols"].as_u64().expect("cast cols") as u16;
    let rows = term["rows"].as_u64().expect("cast rows") as u16;
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut frames = 0;
    for (line_number, line) in lines.enumerate() {
        let event: Value = serde_json::from_str(line)
            .unwrap_or_else(|error| panic!("parse asciinema event {path}:{line_number}: {error}"));
        if event[1].as_str() != Some("o") {
            continue;
        }
        let output = event[2].as_str().expect("output event payload");
        parser.process(output.as_bytes());
        frames += 1;
    }
    assert!(frames > 0, "{path} contains no asciinema frames");
    CastDump {
        name,
        cols,
        rows,
        frames,
        final_screen: parser.screen().contents(),
    }
}

#[test]
fn asciinema_dumps_replay_to_snapshotable_terminal_frames() {
    let dumps = [
        replay_cast("grok-full.cast", "grok-full"),
        replay_cast("grok-rich.cast", "grok-rich"),
        replay_cast("runie-full.cast", "runie-full"),
    ];
    let summary = dumps
        .iter()
        .map(|dump| {
            format!(
                "{}: {}x{}, {} events, final-screen-bytes={}, final-screen-lines={}",
                dump.name,
                dump.cols,
                dump.rows,
                dump.frames,
                dump.final_screen.len(),
                dump.final_screen.lines().count()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("asciinema-dump-replay-summary", summary);
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn grok_casts_have_a_classified_state_for_every_frame() {
    let casts = ["grok-full.cast", "grok-rich.cast"];
    let mut summary = Vec::new();
    for cast_name in casts {
        let cast = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../artifacts")
                .join(cast_name),
        )
        .expect("saved Grok asciinema recording");
        let mut lines = cast.lines();
        let header: Value =
            serde_json::from_str(lines.next().expect("cast header")).expect("cast header json");
        let rows = header["term"]["rows"].as_u64().expect("cast rows") as u16;
        let cols = header["term"]["cols"].as_u64().expect("cast cols") as u16;
        let mut parser = vt100::Parser::new(rows, cols, 0);
        let mut counts = std::collections::BTreeMap::<&str, usize>::new();
        for line in lines {
            let event: Value = serde_json::from_str(line).expect("cast event json");
            if event[1].as_str() != Some("o") {
                continue;
            }
            parser.process(event[2].as_str().expect("output event").as_bytes());
            let screen = parser.screen().contents();
            let state = if screen.contains("Workflows are here!") {
                "welcome"
            } else if screen.contains("Enter:send") && screen.contains("❯") {
                "prompt"
            } else if screen.contains("Waiting for response") {
                "waiting"
            } else if screen.contains("Responding…") {
                "responding"
            } else if screen.contains("Thinking…") {
                "thinking"
            } else if screen.contains("Worked for") {
                "completed"
            } else if screen.contains("Commands")
                || screen.contains("Echo Command Query Title")
                || (screen.contains("New Session") && screen.contains("Keyboard Shortcuts"))
            {
                "command_palette"
            } else if screen.trim().is_empty() {
                "blank"
            } else if screen.contains(" main") {
                "header_only"
            } else {
                "other"
            };
            *counts.entry(state).or_default() += 1;
        }
        assert!(
            !counts.contains_key("other"),
            "unclassified {cast_name} frame(s): {counts:?}"
        );
        summary.push(format!(
            "{cast_name}: {}",
            counts
                .into_iter()
                .map(|(state, count)| format!("{state}={count}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    insta::assert_snapshot!("grok-cast-state-inventory", summary.join("\n"));
}

#[test]
fn grok_cast_reference_captures_full_mode_prompt_chrome() {
    let cast = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/grok-full.cast"),
    )
    .expect("saved Grok asciinema recording");
    let markers = [
        ("top_corner", "╭"),
        ("prompt_cursor", "❯"),
        ("bottom_corner", "╰"),
        ("model_caption", "Grok 4.5"),
        ("send_hint", "Enter"),
        ("mode_hint", "Shift+Tab"),
        ("shortcut_hint", "Ctrl+x"),
    ];
    let summary = markers
        .iter()
        .map(|(name, marker)| format!("{name}: {}", cast.contains(marker)))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("grok-full-mode-prompt-reference", summary);
}

#[test]
fn grok_cast_contract_is_exercised_by_runie_visual_suite() {
    let cast = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/grok-full.cast"),
    )
    .expect("saved Grok asciinema recording");

    // These are the stable, non-timing-dependent contracts observed in every
    // recorded full-mode state. The spinner glyphs themselves are tested by
    // exact frame snapshots in StatusBar; the cast is the source fixture for
    // the vocabulary and geometry contract here.
    let recorded_contracts = [
        ("welcome_frame", "Workflows are here!"),
        ("welcome_actions", "New worktree"),
        ("quit_action", "ctrl+q"),
        ("prompt_top_border", "╭"),
        ("prompt_cursor", "❯"),
        ("prompt_bottom_border", "╰"),
        ("model_caption", "Grok 4.5 (high)"),
        ("idle_send_hint", "Enter"),
        ("mode_hint", "Shift+Tab"),
        ("shortcut_hint", "Ctrl+x"),
        ("active_cancel_hint", "Esc"),
        ("thinking_label", "Thinking"),
        ("reasoning_bar", "┃"),
    ];
    let summary = recorded_contracts
        .iter()
        .map(|(name, marker)| format!("{name}: {}", cast.contains(marker)))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("grok-full-mode-recorded-contracts", summary);
}

#[test]
fn grok_rich_recording_captures_markdown_tool_and_animation_states() {
    let cast = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/grok-rich.cast"),
    )
    .expect("saved rich Grok asciinema recording");
    let contracts = [
        ("welcome", "Workflows are here!"),
        ("markdown_heading", "runie"),
        ("tool_progress", "Listing 1 dir"),
        ("tool_reading", "Reading 1 file"),
        ("tool_glyph", "◈"),
        ("spinner", "⠋"),
        ("prompt_cursor", "❯"),
    ];
    let markdown_bold = cast
        .lines()
        .skip(1)
        .filter_map(|line| {
            serde_json::from_str::<Value>(line)
                .ok()?
                .get(2)?
                .as_str()
                .map(str::to_owned)
        })
        .any(|output| output.contains("\u{001b}[1m"));
    let summary = contracts
        .iter()
        .map(|(name, marker)| format!("{name}: {}", cast.contains(marker)))
        .chain(std::iter::once(format!("markdown_bold: {markdown_bold}")))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("grok-rich-recorded-contracts", summary);
    for (name, marker) in contracts {
        assert!(
            cast.contains(marker),
            "rich Grok recording lacks {name}: {marker:?}"
        );
    }
    assert!(
        markdown_bold,
        "rich Grok recording lacks an ANSI bold markdown span"
    );
}

#[test]
fn grok_cast_ansi_frames_are_exact_instagraphics_references() {
    let cast = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/grok-full.cast"),
    )
    .expect("saved Grok asciinema recording");
    let mut lines = cast.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("cast header")).expect("cast header json");
    let term = header.get("term").expect("cast terminal metadata");
    let cols = term.get("cols").and_then(Value::as_u64).expect("cast cols") as u16;
    let rows = term.get("rows").and_then(Value::as_u64).expect("cast rows") as u16;
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut selected = Vec::new();
    let mut captured = [false; 4];
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        let output = event
            .get(2)
            .and_then(Value::as_str)
            .expect("output event payload");
        parser.process(output.as_bytes());
        let contents = parser.screen().contents();
        let state = if contents.contains("Workflows are here!") {
            Some((0, "welcome"))
        } else if contents.contains("❯ echo") && contents.contains("Enter") {
            Some((1, "typed"))
        } else if contents.contains("Thinking…") {
            Some((2, "thinking"))
        } else if contents.contains("Responding…") {
            Some((3, "responding"))
        } else {
            None
        };
        if let Some((index, name)) = state {
            if !captured[index] {
                captured[index] = true;
                let formatted_bytes = parser.screen().contents_formatted();
                let formatted = String::from_utf8_lossy(&formatted_bytes);
                selected.push(format!(
                    "--- {name} ({cols}x{rows}) ---\n{contents}\n--- ansi ---\n{formatted}"
                ));
            }
        }
    }
    assert!(
        captured.iter().all(|captured| *captured),
        "cast did not contain all stable states"
    );
    insta::assert_snapshot!("grok-full-mode-ansi-frames", selected.join("\n"));
}

#[test]
fn grok_rich_active_footer_is_a_full_width_reference_row() {
    let cast = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/grok-rich.cast"),
    )
    .expect("saved rich Grok recording");
    let mut lines = cast.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("cast header")).expect("cast header json");
    let cols = header["term"]["cols"].as_u64().expect("cast cols") as u16;
    let rows = header["term"]["rows"].as_u64().expect("cast rows") as u16;
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut footer = None;
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        let screen = parser.screen().contents();
        if let Some(row) = screen
            .lines()
            .find(|row| row.contains("Esc") && row.contains("shortcuts"))
        {
            footer = Some(row.to_owned());
        }
    }
    let mut footer = footer.expect("rich cast active footer");
    footer.extend(std::iter::repeat(' ').take(cols as usize - footer.chars().count()));
    insta::assert_snapshot!("grok-rich-active-footer-row", footer);
}
