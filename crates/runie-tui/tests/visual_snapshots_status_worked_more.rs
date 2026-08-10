#![cfg(any())]

use super::*;
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
