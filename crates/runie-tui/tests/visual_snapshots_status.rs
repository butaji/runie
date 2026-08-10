#![cfg(any())]

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

#[path = "visual_snapshots_status_matrix.rs"]
mod status_matrix;
#[path = "visual_snapshots_status_worked.rs"]
mod status_worked;
use super::*;
