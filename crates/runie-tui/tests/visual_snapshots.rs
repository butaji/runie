#![allow(
    clippy::manual_repeat_n,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    reason = "fixture comparisons intentionally keep each recorded frame assertion together"
)]

use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use runie_core::types::ThemeKind;
use serde_json::Value;

use runie_tui::widgets::{
    braille_spinner_frames, Line, LineKind, Scrollback, Status, StatusBar, TurnStatus,
    TurnStatusPhase,
};
use runie_tui::yaml_runner::{load_scenario, render_visual, render_visual_buffer};

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

#[test]
#[allow(clippy::cognitive_complexity)]
fn runie_active_footer_matches_grok_cast_cells_and_bold_keys() {
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
    let mut expected_rows = Vec::new();
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        if let Some(row) = parser
            .screen()
            .contents()
            .lines()
            .find(|row| row.contains("Esc") && row.contains("shortcuts"))
        {
            let mut row = row.to_owned();
            row.extend(std::iter::repeat(' ').take(cols as usize - row.chars().count()));
            expected_rows.push(row);
        }
    }
    let expected = expected_rows.last().cloned().expect("Grok active footer");
    let mut status = StatusBar::new();
    status.set(Status::Thinking);
    let mut buffer = Buffer::empty(Rect::new(0, 0, cols, 1));
    status.render(Rect::new(2, 0, cols - 4, 1), &mut buffer);
    for (x, expected_symbol) in expected.chars().enumerate() {
        let cell = buffer.cell((x as u16, 0)).expect("Runie footer cell");
        assert_eq!(
            cell.symbol().chars().next().unwrap_or(' '),
            expected_symbol,
            "footer cell {x}"
        );
    }
    for (frame, row) in expected_rows.iter().enumerate() {
        for (x, expected_symbol) in row.chars().enumerate() {
            assert_eq!(
                buffer
                    .cell((x as u16, 0))
                    .expect("Runie footer cell")
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' '),
                expected_symbol,
                "footer frame {frame} cell {x}"
            );
        }
    }
    for key in ["Shift+Tab", "Esc", "Ctrl+."] {
        let start = expected
            .chars()
            .collect::<Vec<_>>()
            .windows(key.chars().count())
            .position(|window| window.iter().copied().eq(key.chars()))
            .expect("Grok footer key") as u16;
        for x in start..start + key.chars().count() as u16 {
            assert!(
                buffer
                    .cell((x, 0))
                    .expect("Runie key cell")
                    .modifier
                    .contains(Modifier::BOLD),
                "Runie footer key {key} cell {x} is not bold"
            );
        }
    }
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn runie_turn_status_matches_recorded_grok_starting_session_row() {
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
    let mut expected_rows = Vec::new();
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        if let Some(row) = parser
            .screen()
            .contents()
            .lines()
            .find(|row| row.contains("Starting session…"))
        {
            expected_rows.push(row.to_owned());
        }
    }
    let mut expected = expected_rows
        .first()
        .cloned()
        .expect("Grok starting-session row");
    expected.extend(std::iter::repeat(' ').take(cols as usize - expected.chars().count()));
    let mut buffer = Buffer::empty(Rect::new(0, 0, cols, 1));
    TurnStatus::new(0).render(Rect::new(2, 0, cols - 4, 1), &mut buffer);
    for (x, expected_symbol) in expected.chars().enumerate() {
        let actual = buffer
            .cell((x as u16, 0))
            .expect("Runie status cell")
            .symbol();
        assert_eq!(
            actual.chars().next().unwrap_or(' '),
            expected_symbol,
            "status cell {x}"
        );
    }
    let grok_status_cell = parser.screen().cell(16, 2).expect("Grok status cell");
    let runie_status_cell = buffer.cell((4, 0)).expect("Runie spinner cell");
    assert_eq!(format!("{:?}", grok_status_cell.fgcolor()), "Default");
    assert!(!grok_status_cell.bold(), "Grok status row must not be bold");
    assert_eq!(format!("{:?}", runie_status_cell.fg), "Rgb(187, 154, 247)");
    assert!(
        !runie_status_cell.modifier.contains(Modifier::DIM),
        "Runie status row must use role colors, not blanket DIM"
    );
    assert_eq!(expected.chars().count(), cols as usize);
    for (frame, row) in expected_rows.iter().enumerate() {
        let mut row = row.clone();
        row.extend(std::iter::repeat(' ').take(cols as usize - row.chars().count()));
        let spinner = row.chars().nth(4).expect("starting spinner cell");
        let spinner_frame = braille_spinner_frames()
            .iter()
            .position(|candidate| candidate.starts_with(spinner))
            .expect("known starting spinner frame");
        let mut frame_buffer = Buffer::empty(Rect::new(0, 0, cols, 1));
        TurnStatus::new(spinner_frame * 3).render(Rect::new(2, 0, cols - 4, 1), &mut frame_buffer);
        let stable_width = format!("  {spinner} Starting session…").chars().count();
        for (x, expected_symbol) in row.chars().take(stable_width).enumerate() {
            assert_eq!(
                frame_buffer
                    .cell((x as u16, 0))
                    .expect("Runie status cell")
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' '),
                expected_symbol,
                "starting status frame {frame} cell {x}"
            );
        }
    }
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn runie_waiting_status_matches_recorded_grok_waiting_row() {
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
    let mut expected_rows = Vec::new();
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        if let Some(row) = parser
            .screen()
            .contents()
            .lines()
            .find(|row| row.contains("Waiting for response…"))
        {
            expected_rows.push(row.to_owned());
        }
    }
    let mut expected = expected_rows.first().cloned().expect("Grok waiting row");
    expected.extend(std::iter::repeat(' ').take(cols as usize - expected.chars().count()));
    let mut buffer = Buffer::empty(Rect::new(0, 0, cols, 1));
    TurnStatus::new(12)
        .phase(TurnStatusPhase::Waiting)
        .with_chrome(" 0.0s                            0.0s ⇣3.18k [stop]")
        .render(Rect::new(2, 0, cols - 4, 1), &mut buffer);
    for (x, expected_symbol) in expected.chars().enumerate() {
        assert_eq!(
            buffer
                .cell((x as u16, 0))
                .expect("Runie waiting cell")
                .symbol()
                .chars()
                .next()
                .unwrap_or(' '),
            expected_symbol,
            "waiting status cell {x}"
        );
    }
    for (frame, row) in expected_rows.iter().enumerate() {
        let mut row = row.clone();
        row.extend(std::iter::repeat(' ').take(cols as usize - row.chars().count()));
        let spinner = row.chars().nth(4).expect("waiting spinner cell");
        let spinner_frame = braille_spinner_frames()
            .iter()
            .position(|candidate| candidate.starts_with(spinner))
            .expect("known waiting spinner frame");
        let mut frame_buffer = Buffer::empty(Rect::new(0, 0, cols, 1));
        TurnStatus::new(spinner_frame * 3)
            .phase(TurnStatusPhase::Waiting)
            .with_chrome(" 0.0s                            0.0s ⇣3.18k [stop]")
            .render(Rect::new(2, 0, cols - 4, 1), &mut frame_buffer);
        let stable_width = format!("  {spinner} Waiting for response…").chars().count();
        for (x, expected_symbol) in row.chars().take(stable_width).enumerate() {
            assert_eq!(
                frame_buffer
                    .cell((x as u16, 0))
                    .expect("Runie waiting cell")
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' '),
                expected_symbol,
                "waiting status frame {frame} cell {x}"
            );
        }
    }
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn runie_responding_status_matches_recorded_full_mode_row() {
    let cast = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/grok-full.cast"),
    )
    .expect("saved full Grok recording");
    let mut lines = cast.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("cast header")).expect("cast header json");
    let cols = header["term"]["cols"].as_u64().expect("cast cols") as u16;
    let rows = header["term"]["rows"].as_u64().expect("cast rows") as u16;
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut expected_rows = Vec::new();
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        if let Some(row) = parser
            .screen()
            .contents()
            .lines()
            .find(|row| row.contains("Responding…"))
        {
            expected_rows.push(row.to_owned());
        }
    }
    let mut expected = expected_rows.first().cloned().expect("Grok responding row");
    expected.extend(std::iter::repeat(' ').take(cols as usize - expected.chars().count()));
    let mut buffer = Buffer::empty(Rect::new(0, 0, cols, 1));
    TurnStatus::new(9)
        .phase(TurnStatusPhase::Responding)
        .with_chrome(" 0.0s                                                                              2.3s ⇣6.39k [stop]")
        .render(Rect::new(2, 0, cols - 4, 1), &mut buffer);
    for (x, expected_symbol) in expected.chars().enumerate() {
        assert_eq!(
            buffer
                .cell((x as u16, 0))
                .expect("Runie responding cell")
                .symbol()
                .chars()
                .next()
                .unwrap_or(' '),
            expected_symbol,
            "responding status cell {x}"
        );
    }
    for (frame, row) in expected_rows.iter().enumerate() {
        let mut row = row.clone();
        row.extend(std::iter::repeat(' ').take(cols as usize - row.chars().count()));
        let spinner = row.chars().nth(4).expect("responding spinner cell");
        let spinner_frame = braille_spinner_frames()
            .iter()
            .position(|candidate| candidate.starts_with(spinner))
            .expect("known responding spinner frame");
        let mut frame_buffer = Buffer::empty(Rect::new(0, 0, cols, 1));
        TurnStatus::new(spinner_frame * 3)
            .phase(TurnStatusPhase::Responding)
            .with_chrome(
                " 0.0s                                                                              2.3s ⇣6.39k [stop]",
            )
            .render(Rect::new(2, 0, cols - 4, 1), &mut frame_buffer);
        for (x, expected_symbol) in row.chars().enumerate() {
            assert_eq!(
                frame_buffer
                    .cell((x as u16, 0))
                    .expect("Runie responding cell")
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' '),
                expected_symbol,
                "responding status frame {frame} cell {x}"
            );
        }
    }
}

#[test]
fn turn_status_phase_matrix_snapshot() {
    let phases = [
        ("starting", TurnStatus::new(0)),
        ("waiting", TurnStatus::new(4).phase(TurnStatusPhase::Waiting).with_chrome(" 0.0s                            0.0s ⇣3.18k [stop]")),
        ("thinking", TurnStatus::new(2).phase(TurnStatusPhase::Thinking)),
        ("responding", TurnStatus::new(3).phase(TurnStatusPhase::Responding).with_chrome(" 0.0s                                                                              2.3s ⇣6.39k [stop]")),
    ];
    insta::assert_snapshot!(phases
        .iter()
        .map(|(name, status)| format!("{name}: {}", status.text()))
        .collect::<Vec<_>>()
        .join("\n"));
}

#[test]
fn worked_for_row_matches_grok_cast_cells() {
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
    let mut expected = None;
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        if let Some(row) = parser
            .screen()
            .contents()
            .lines()
            .find(|row| row.contains("Worked for"))
        {
            expected = Some(row.trim_end().to_owned());
        }
    }
    let expected = expected.expect("Grok completion row");
    let duration = expected.trim().to_owned();
    let mut scrollback = Scrollback::new();
    scrollback.append(Line::new(LineKind::TurnSummary, duration));
    let mut buffer = Buffer::empty(Rect::new(0, 0, cols, 1));
    scrollback.render(Rect::new(0, 0, cols, 1), &mut buffer);
    let actual = (0..cols)
        .map(|x| {
            buffer
                .cell((x, 0))
                .expect("Runie completion cell")
                .symbol()
                .chars()
                .next()
                .unwrap_or(' ')
        })
        .collect::<String>()
        .trim_end()
        .to_owned();
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn runie_idle_welcome_matches_grok_cast_cells() {
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
    let mut grok = None;
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        if parser.screen().contents().contains("Workflows are here!") {
            grok = Some(parser.screen().contents());
            break;
        }
    }
    let grok = grok.expect("Grok welcome frame");
    let scenario = load_scenario(&fixture("visual-prompts.yaml")).expect("idle fixture");
    let mut visual = scenario.assertions.visual.clone().expect("idle visual");
    visual.cols = cols;
    visual.rows = rows;
    let runie = render_visual_buffer(&scenario, &visual)
        .await
        .expect("Runie welcome frame");
    let grok_rows: Vec<Vec<char>> = grok.lines().map(|line| line.chars().collect()).collect();
    for y in 0..18 {
        for x in 0..cols as usize {
            let expected = grok_rows[y].get(x).copied().unwrap_or(' ');
            let actual = runie
                .cell((x as u16, y as u16))
                .expect("Runie welcome cell")
                .symbol()
                .chars()
                .next()
                .unwrap_or(' ');
            assert_eq!(actual, expected, "welcome cell mismatch at ({x},{y})");
        }
    }

    // Compare stable ANSI attributes as well as symbols. Grok's welcome
    // actions are bold, while the branch/path header is dim and the body is
    // plain. These are cell-level invariants, not substring checks.
    let grok_screen = parser.screen();
    let grok_action = grok_screen.cell(5, 15).expect("Grok action cell");
    let runie_action = runie.cell((15, 5)).expect("Runie action cell");
    assert_eq!(
        grok_action.bold(),
        runie_action.modifier.contains(Modifier::BOLD)
    );
    let grok_header = grok_screen.cell(1, 2).expect("Grok header cell");
    let runie_header = runie.cell((2, 1)).expect("Runie header cell");
    assert_eq!(
        grok_header.bold(),
        runie_header.modifier.contains(Modifier::BOLD)
    );
    let grok_body = grok_screen.cell(10, 15).expect("Grok body cell");
    let runie_body = runie.cell((15, 10)).expect("Runie body cell");
    assert_eq!(
        grok_body.bold(),
        runie_body.modifier.contains(Modifier::BOLD)
    );
}

#[tokio::test]
async fn every_grok_welcome_frame_matches_runie_stable_cells() {
    let cast = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/grok-rich.cast"),
    )
    .expect("saved rich Grok recording");
    let mut lines = cast.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("cast header")).expect("cast header json");
    let cols = header["term"]["cols"].as_u64().expect("cast cols") as u16;
    let rows = header["term"]["rows"].as_u64().expect("cast rows") as u16;
    let scenario = load_scenario(&fixture("visual-prompts.yaml")).expect("idle fixture");
    let mut visual = scenario.assertions.visual.clone().expect("idle visual");
    visual.cols = cols;
    visual.rows = rows;
    let runie = render_visual_buffer(&scenario, &visual)
        .await
        .expect("Runie welcome frame");
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut matched = 0usize;
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        let screen = parser.screen().contents();
        if !screen.contains("Workflows are here!") {
            continue;
        }
        matched += 1;
        let grok_rows: Vec<Vec<char>> = screen.lines().map(|row| row.chars().collect()).collect();
        // Rows 0-14 are the stable welcome surface. Grok replaces the
        // lower tip/prompt chrome while retaining the welcome content.
        for y in 0..15 {
            for x in 0..cols as usize {
                let expected = grok_rows[y].get(x).copied().unwrap_or(' ');
                let actual = runie
                    .cell((x as u16, y as u16))
                    .expect("Runie welcome cell")
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ');
                assert_eq!(
                    actual, expected,
                    "welcome frame {matched} cell mismatch at ({x},{y})"
                );
            }
        }
    }
    assert!(
        matched > 1,
        "Grok cast must contain multiple welcome frames"
    );
}

#[tokio::test]
async fn grok_typed_prompt_geometry_matches_runie_cells() {
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
    let mut grok_typed = None;
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        let output = event
            .get(2)
            .and_then(Value::as_str)
            .expect("output payload");
        parser.process(output.as_bytes());
        let contents = parser.screen().contents();
        if contents.contains("❯ echo") && contents.contains("Enter") {
            grok_typed = Some(contents);
            break;
        }
    }
    let grok = grok_typed.expect("typed Grok frame");
    let grok_rows: Vec<Vec<char>> = grok.lines().map(|line| line.chars().collect()).collect();
    let top = grok_rows
        .windows(2)
        .position(|window| window[0].contains(&'╭') && window[1].contains(&'❯'))
        .expect("Grok prompt top border");
    let left = grok_rows[top]
        .iter()
        .position(|c| *c == '╭')
        .expect("Grok prompt left corner");
    let bottom = grok_rows
        .iter()
        .enumerate()
        .skip(top + 1)
        .find(|(_, line)| line.get(left) == Some(&'╰'))
        .map(|(index, _)| index)
        .expect("Grok prompt bottom border");
    let footer = grok_rows
        .iter()
        .position(|line| line.windows(5).any(|w| w == ['E', 'n', 't', 'e', 'r']))
        .expect("Grok idle footer");

    let scenario = load_scenario(&fixture("visual-typed.yaml")).expect("typed fixture");
    let mut visual = scenario.assertions.visual.clone().expect("typed visual");
    visual.cols = cols;
    visual.rows = rows;
    let runie = render_visual_buffer(&scenario, &visual)
        .await
        .expect("Runie buffer");
    for y in 0..=footer {
        for x in 0..cols as usize {
            let expected = grok_rows[y].get(x).copied().unwrap_or(' ');
            let actual = runie
                .cell((x as u16, y as u16))
                .expect("Runie cell")
                .symbol();
            let actual = actual.chars().next().unwrap_or(' ');
            assert_eq!(actual, expected, "cell mismatch at ({x},{y})");
        }
    }
    assert_eq!(
        runie
            .cell((left as u16, top as u16))
            .expect("Runie top-left")
            .symbol(),
        "╭"
    );
    assert_eq!(
        runie
            .cell((left as u16, bottom as u16))
            .expect("Runie bottom-left")
            .symbol(),
        "╰"
    );
    assert_eq!(
        runie
            .cell((left as u16 + 1, top as u16))
            .expect("Runie top edge")
            .symbol(),
        "─"
    );
    assert_eq!(
        runie
            .cell((left as u16 + 1, bottom as u16))
            .expect("Runie bottom edge")
            .symbol(),
        "─"
    );

    for label in ["Enter", "Shift+Tab", "Ctrl+x"] {
        let x = grok_rows[footer]
            .windows(label.chars().count())
            .position(|window| window.iter().copied().eq(label.chars()))
            .expect("Grok footer label") as u16;
        let cell = runie.cell((x, footer as u16)).expect("Runie footer cell");
        assert_eq!(cell.symbol(), &label[0..1]);
        assert_eq!(
            cell.fg,
            runie_tui::appearance::footer_key_style_for(ThemeKind::GrokNight)
                .fg
                .expect("footer key color"),
            "{label} foreground"
        );
        assert!(
            cell.modifier.contains(ratatui::style::Modifier::BOLD),
            "{label} bold"
        );
    }
    for x in left..cols as usize {
        for y in [top, bottom] {
            let cell = runie.cell((x as u16, y as u16)).expect("Runie border cell");
            if ["─", "╭", "╮", "╰", "╯"].contains(&cell.symbol()) {
                assert_eq!(
                    cell.fg,
                    runie_tui::appearance::prompt_border_style_for(ThemeKind::GrokNight)
                        .fg
                        .expect("muted token"),
                    "border foreground at ({x},{y})"
                );
                assert!(cell.modifier.is_empty(), "border modifiers at ({x},{y})");
            }
        }
    }
    for y in (top + 1)..bottom {
        for x in [left, cols as usize - 3] {
            let cell = runie.cell((x as u16, y as u16)).expect("Runie side cell");
            assert_eq!(
                cell.fg,
                runie_tui::appearance::prompt_border_style_for(ThemeKind::GrokNight)
                    .fg
                    .expect("prompt border color"),
                "side foreground at ({x},{y})"
            );
            assert!(cell.modifier.is_empty(), "side modifiers at ({x},{y})");
        }
    }
}

#[tokio::test]
async fn grok_feed_yaml_matches_recorded_transcript_gutter_cells() {
    let scenario = load_scenario(&fixture("visual-grok-feed.yaml")).expect("Grok feed fixture");
    let visual = scenario
        .assertions
        .visual
        .clone()
        .expect("feed visual assertions");
    let buffer = render_visual_buffer(&scenario, &visual)
        .await
        .expect("Runie feed buffer");
    let row = |y: u16| -> String {
        (0..visual.cols)
            .map(|x| buffer.cell((x, y)).expect("buffer cell").symbol())
            .collect()
    };
    assert_eq!(row(0), " ".repeat(visual.cols as usize));
    assert!(row(1).starts_with("   main ~/Code/GitHub/runie-tests/runie"));
    let activity_row = (0..visual.rows)
        .find(|y| row(*y).contains("◈ Listed 1 dir, Read 1 file"))
        .expect("Grok grouped tool activity row");
    assert_eq!(row(activity_row).chars().nth(2), Some('❙'));
    assert_eq!(row(activity_row).chars().nth(5), Some('◈'));
    let activity_prefix = "  ❙  ◈ Listed 1 dir, Read 1 file";
    let expected_activity = format!(
        "{activity_prefix}{:width$}",
        "",
        width = visual.cols as usize - activity_prefix.chars().count()
    );
    assert_eq!(row(activity_row), expected_activity, "grouped activity row");

    let cast = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/grok-rich.cast"),
    )
    .expect("saved rich Grok recording");
    let mut cast_lines = cast.lines();
    let header: Value =
        serde_json::from_str(cast_lines.next().expect("cast header")).expect("cast header json");
    let cast_cols = header["term"]["cols"].as_u64().expect("cast cols") as u16;
    let cast_rows = header["term"]["rows"].as_u64().expect("cast rows") as u16;
    let mut parser = vt100::Parser::new(cast_rows, cast_cols, 0);
    let mut recorded_activity = None;
    for line in cast_lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        if let Some(recorded) = parser
            .screen()
            .contents()
            .lines()
            .find(|line| line.contains("Listed 1 dir, Read 1 file"))
        {
            recorded_activity = Some(recorded.to_owned());
        }
    }
    let recorded_activity = recorded_activity
        .expect("Grok grouped activity row")
        .trim_end_matches('█')
        .to_owned();
    assert_eq!(cast_cols, visual.cols, "feed and cast column geometry");
    assert_eq!(
        row(activity_row),
        format!("{recorded_activity:<width$}", width = visual.cols as usize),
        "YAML activity row must match the recorded Grok cast"
    );
    for output in ["Cargo.toml", "src", "crates"] {
        assert!(
            (0..visual.rows).all(|y| !row(y).starts_with(&format!("    {output}"))),
            "collapsed Grok feed must hide member output row {output}"
        );
    }
    for y in 0..visual.rows {
        assert_eq!(
            row(y).chars().count(),
            visual.cols as usize,
            "row width {y}"
        );
    }
}

async fn snapshot_fixture(name: &str) {
    let scenario = load_scenario(&fixture(name)).expect("load visual fixture");
    let visual = scenario
        .assertions
        .visual
        .as_ref()
        .expect("visual assertions");
    let screen = render_visual(&scenario, visual)
        .await
        .expect("render visual fixture");
    let snapshot_name = name.strip_suffix(".yaml").unwrap_or(name);
    insta::assert_snapshot!(snapshot_name, screen);
}

#[tokio::test]
async fn grok_recorded_symbols_render_in_runie_frames() {
    let cast = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/grok-full.cast"),
    )
    .expect("saved Grok asciinema recording");
    let symbols = ["╭", "╮", "╰", "╯", "❯", "Enter", "Shift+Tab", "Ctrl+x"];
    for symbol in symbols {
        assert!(
            cast.contains(symbol),
            "Grok recording lacks required symbol {symbol:?}"
        );
    }

    let typed = load_scenario(&fixture("visual-typed.yaml")).expect("typed fixture");
    let mut typed_visual = typed
        .assertions
        .visual
        .as_ref()
        .expect("typed visual")
        .clone();
    // Use the captured 120-column × 36-row geometry when checking the full
    // footer rather than truncating it.
    typed_visual.cols = 120;
    typed_visual.rows = 36;
    let typed_screen = render_visual(&typed, &typed_visual)
        .await
        .expect("typed render");
    for symbol in [
        "╭",
        "╮",
        "╰",
        "╯",
        "❯",
        "Grok 4.5 (high)",
        "Enter:send",
        "Shift+Tab:mode",
        "Ctrl+x:shortcuts",
    ] {
        assert!(
            typed_screen.contains(symbol),
            "Runie typed frame lacks {symbol:?}:\n{typed_screen}"
        );
    }

    let reasoning = load_scenario(&fixture("visual-reasoning.yaml")).expect("reasoning fixture");
    let reasoning_visual = reasoning
        .assertions
        .visual
        .as_ref()
        .expect("reasoning visual");
    let reasoning_screen = render_visual(&reasoning, reasoning_visual)
        .await
        .expect("reasoning render");
    assert!(
        reasoning_screen.contains("┃"),
        "Runie reasoning frame lacks Grok's vertical bar"
    );
}

#[tokio::test]
async fn full_mode_idle_snapshot() {
    snapshot_fixture("visual-prompts.yaml").await;
}

#[tokio::test]
async fn full_mode_typed_prompt_snapshot() {
    snapshot_fixture("visual-typed.yaml").await;
}

#[tokio::test]
async fn full_mode_submitted_snapshot() {
    snapshot_fixture("visual-submitted.yaml").await;
}

#[tokio::test]
async fn full_mode_reasoning_snapshot() {
    snapshot_fixture("visual-reasoning.yaml").await;
}

#[tokio::test]
async fn full_mode_error_snapshot() {
    snapshot_fixture("visual-error.yaml").await;
}

#[tokio::test]
async fn full_mode_tool_snapshot() {
    snapshot_fixture("visual-tool.yaml").await;
}

#[tokio::test]
async fn full_mode_resize_snapshot() {
    snapshot_fixture("visual-resize.yaml").await;
}

#[tokio::test]
async fn full_mode_scroll_snapshot() {
    snapshot_fixture("visual-scroll.yaml").await;
}

#[tokio::test]
async fn grok_waiting_capture_uses_event_boundary() {
    let scenario =
        load_scenario(&fixture("visual-grok-waiting.yaml")).expect("Grok waiting fixture");
    let visual = scenario
        .assertions
        .visual
        .as_ref()
        .expect("Grok waiting visual assertions");
    let screen = render_visual(&scenario, visual)
        .await
        .expect("Grok waiting frame");
    assert!(
        screen.contains("◈ Listed 1 dir"),
        "missing grouped activity"
    );
    assert!(
        !screen.contains("(run finished"),
        "captured completed state"
    );
}
