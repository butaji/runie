use super::*;

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

#[path = "visual_snapshots_status_worked_more.rs"]
mod worked_more;
