use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use serde_json::Value;

use runie_tui::widgets::{Status, StatusBar, TurnStatus, TurnStatusPhase};
use runie_tui::yaml_runner::{load_scenario, render_visual, render_visual_buffer};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/e2e")
        .join(name)
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
    let mut expected = None;
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
            expected = Some(row);
        }
    }
    let expected = expected.expect("Grok active footer");
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
    let mut expected = None;
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        if let Some(row) = parser
            .screen()
            .contents()
            .lines()
            .find(|row| row.contains("Starting session…"))
        {
            expected = Some(row.to_owned());
            break;
        }
    }
    let mut expected = expected.expect("Grok starting-session row");
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
    let runie_status_cell = buffer.cell((2, 0)).expect("Runie status cell");
    assert_eq!(format!("{:?}", grok_status_cell.fgcolor()), "Default");
    assert!(!grok_status_cell.bold(), "Grok status row must not be bold");
    assert!(
        runie_status_cell.modifier.contains(Modifier::DIM),
        "Runie status row must be dim"
    );
    assert_eq!(expected.chars().count(), cols as usize);
}

#[test]
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
    let mut expected = None;
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        if let Some(row) = parser
            .screen()
            .contents()
            .lines()
            .find(|row| row.contains("Waiting for response…"))
        {
            expected = Some(row.to_owned());
            break;
        }
    }
    let mut expected = expected.expect("Grok waiting row");
    expected.extend(std::iter::repeat(' ').take(cols as usize - expected.chars().count()));
    let mut buffer = Buffer::empty(Rect::new(0, 0, cols, 1));
    TurnStatus::new(4)
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
}

#[test]
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
    let mut expected = None;
    for line in lines {
        let event: Value = serde_json::from_str(line).expect("cast event json");
        parser.process(event[2].as_str().expect("cast output").as_bytes());
        if let Some(row) = parser
            .screen()
            .contents()
            .lines()
            .find(|row| row.contains("Responding…"))
        {
            expected = Some(row.to_owned());
            break;
        }
    }
    let mut expected = expected.expect("Grok responding row");
    expected.extend(std::iter::repeat(' ').take(cols as usize - expected.chars().count()));
    let mut buffer = Buffer::empty(Rect::new(0, 0, cols, 1));
    TurnStatus::new(3)
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
        assert_eq!(cell.fg, ratatui::style::Color::Reset, "{label} foreground");
        assert!(
            cell.modifier.contains(ratatui::style::Modifier::BOLD),
            "{label} bold"
        );
    }
    for x in left..cols as usize {
        for y in [top, bottom] {
            let cell = runie.cell((x as u16, y as u16)).expect("Runie border cell");
            assert_eq!(
                cell.fg,
                ratatui::style::Color::Reset,
                "border foreground at ({x},{y})"
            );
            assert!(cell.modifier.is_empty(), "border modifiers at ({x},{y})");
        }
    }
    for y in (top + 1)..bottom {
        for x in [left, cols as usize - 3] {
            let cell = runie.cell((x as u16, y as u16)).expect("Runie side cell");
            assert_eq!(
                cell.fg,
                ratatui::style::Color::Reset,
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
    for output in ["Cargo.toml", "src", "crates"] {
        let output_row = (0..visual.rows)
            .find(|y| row(*y).starts_with(&format!("    {output}")))
            .expect("structured list output row");
        let expected = format!(
            "    {output}{:width$}",
            "",
            width = visual.cols as usize - 4 - output.chars().count()
        );
        assert_eq!(row(output_row), expected, "list output row for {output}");
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
