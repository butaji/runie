#![cfg(any())]

use super::*;
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

#[path = "visual_snapshots_status.rs"]
mod status_snapshots;
