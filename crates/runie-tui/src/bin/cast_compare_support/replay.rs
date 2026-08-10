use super::*;
pub(crate) fn parse_marker(marker: Option<&str>) -> (&str, usize) {
    marker
        .and_then(|value| {
            value
                .rsplit_once('#')
                .and_then(|(text, occurrence)| occurrence.parse::<usize>().ok().map(|n| (text, n)))
        })
        .unwrap_or((marker.unwrap_or_default(), 1))
}

pub(crate) fn marker_is_reached(
    parser: &vt100::Parser,
    marker_texts: &[&str],
    marker_visible: &mut bool,
    seen_markers: &mut usize,
    marker_occurrence: usize,
) -> bool {
    let contains_marker = marker_texts
        .iter()
        .all(|text| parser.screen().contents().contains(text));
    if contains_marker && !*marker_visible {
        *seen_markers += 1;
    }
    *marker_visible = contains_marker;
    *seen_markers >= marker_occurrence
}

pub(crate) fn append_changed_frame(
    parser: &vt100::Parser,
    rows: u16,
    cols: u16,
    previous: &mut Option<Vec<Cell>>,
    frames: &mut Vec<Vec<Cell>>,
) {
    let frame = cells(parser, rows, cols);
    if previous.as_ref() != Some(&frame) {
        *previous = Some(frame.clone());
        frames.push(frame);
    }
}

pub(crate) fn replay(path: &Path) -> Result<Replay> {
    let content = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let has_alternate_screen = content.contains("\u{1b}[?1049h");
    let mut lines = content.lines().peekable();
    let header: Value = serde_json::from_str(lines.next().context("cast header")?)?;
    let (cols, rows) = dimensions(&header)?;
    let mut parser = vt100::Parser::new(rows, cols, 0);
    replay_output_events(&mut lines, &mut parser, has_alternate_screen)?;
    let screen = parser.screen();
    let current_cells = cells(&parser, rows, cols);
    let current_contents = screen.contents().lines().map(str::to_owned).collect();
    Ok(((cols, rows), current_cells, current_contents))
}

pub(crate) fn replay_output_events(
    lines: &mut std::iter::Peekable<std::str::Lines<'_>>,
    parser: &mut vt100::Parser,
    has_alternate_screen: bool,
) -> Result<()> {
    let mut entered_alternate_screen = false;
    while let Some(line) = lines.next() {
        let event: Value = serde_json::from_str(line)?;
        if event[1].as_str() != Some("o") {
            continue;
        }
        let raw_output = event[2].as_str().context("output payload")?;
        let (output, exited_alternate_screen) = normalize_cast_output(raw_output);
        if output.contains("\u{1b}[2J") && lines.peek().is_some_and(|next| next.contains("?1049l"))
        {
            break;
        }
        if output.contains("\u{1b}[?1049h") {
            entered_alternate_screen = true;
        } else if has_alternate_screen && !entered_alternate_screen {
            continue;
        }
        parser.process(strip_private_modes(&output).as_bytes());
        if exited_alternate_screen {
            break;
        }
    }
    Ok(())
}

pub(crate) fn normalize_cast_output(raw: &str) -> (String, bool) {
    let output = raw.replace("\u{1b}[?1049h", "");
    let exited = output.contains("\u{1b}[?1049l");
    let output = output
        .split_once("\u{1b}[?1049l")
        .map_or(output.clone(), |(before_exit, _)| before_exit.to_owned());
    (output, exited)
}
