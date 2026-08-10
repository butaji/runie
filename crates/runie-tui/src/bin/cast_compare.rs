//! Compare two asciinema casts after replaying them through the same VT parser.

use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Cell {
    symbol: String,
    width: u8,
    fg: String,
    bg: String,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
}

type Replay = ((u16, u16), Vec<Cell>, Vec<String>);
type FrameReplay = ((u16, u16), Vec<Vec<Cell>>);

/// Count frames that occur in the same order in both captures. This is a
/// diagnostic only: strict parity still requires equal frame counts and
/// ordinal cell equality. The ordered intersection separates capture cadence
/// differences from frames that have no visual counterpart at all.
fn ordered_common_frame_count(left: &[Vec<Cell>], right: &[Vec<Cell>]) -> usize {
    let mut right_index = 0;
    let mut common = 0;
    for left_frame in left {
        if let Some(offset) = right[right_index..]
            .iter()
            .position(|right_frame| right_frame == left_frame)
        {
            right_index += offset + 1;
            common += 1;
        }
    }
    common
}

fn frame_cell_difference_counts(left: &[Vec<Cell>], right: &[Vec<Cell>]) -> Vec<usize> {
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            left.iter()
                .zip(right)
                .filter(|(left, right)| left != right)
                .count()
                + left.len().abs_diff(right.len())
        })
        .collect()
}

fn strip_private_modes(output: &str) -> String {
    let bytes = output.as_bytes();
    let mut result = String::with_capacity(output.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"\x1b[?") {
            index += 3;
            while index < bytes.len() && !bytes[index].is_ascii_alphabetic() {
                index += 1;
            }
            index += usize::from(index < bytes.len());
        } else {
            let character = output[index..].chars().next().expect("UTF-8 character");
            result.push(character);
            index += character.len_utf8();
        }
    }
    result
}

fn color_key(color: vt100::Color) -> String {
    match color {
        vt100::Color::Default => "default".into(),
        vt100::Color::Idx(value) => format!("idx:{value}"),
        vt100::Color::Rgb(red, green, blue) => format!("rgb:{red},{green},{blue}"),
    }
}

fn dimensions(header: &Value) -> Result<(u16, u16)> {
    fn dimension(value: Option<u64>, name: &str) -> Result<u16> {
        let value = value.with_context(|| format!("cast {name}"))?;
        u16::try_from(value).with_context(|| format!("cast {name} exceeds u16"))
    }
    if let Some(term) = header.get("term") {
        return Ok((
            dimension(term["cols"].as_u64(), "term.cols")?,
            dimension(term["rows"].as_u64(), "term.rows")?,
        ));
    }
    Ok((
        dimension(header["width"].as_u64(), "width")?,
        dimension(header["height"].as_u64(), "height")?,
    ))
}

fn validate_capture_metadata(left: &Path, right: &Path) -> Result<()> {
    let left_path = left.with_extension("meta.json");
    let right_path = right.with_extension("meta.json");
    let left_exists = left_path.exists();
    let right_exists = right_path.exists();
    if left_exists != right_exists {
        bail!(
            "paired capture metadata is incomplete: {} / {}",
            left_path.display(),
            right_path.display()
        );
    }
    if !left_exists {
        return Ok(());
    }
    let left_meta = load_capture_metadata(&left_path)?;
    let right_meta = load_capture_metadata(&right_path)?;
    validate_capture_metadata_shape(&left_meta, &left_path)?;
    validate_capture_metadata_shape(&right_meta, &right_path)?;
    validate_capture_artifacts(&left_meta, &left_path)?;
    validate_capture_artifacts(&right_meta, &right_path)?;
    validate_resize_artifact(&left_meta, &left_path)?;
    validate_resize_artifact(&right_meta, &right_path)?;
    compare_capture_metadata_fields(&left_meta, &right_meta)
}

fn load_capture_metadata(path: &Path) -> Result<Value> {
    serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| path.display().to_string())?,
    )
    .context("capture metadata JSON")
}

fn compare_capture_metadata_fields(left_meta: &Value, right_meta: &Value) -> Result<()> {
    for (path, label) in [
        ("probe.prompt", "prompt"),
        ("terminal.cols", "terminal columns"),
        ("terminal.rows", "terminal rows"),
        ("terminal.term", "TERM"),
        ("terminal.colorterm", "COLORTERM"),
        ("probe.quit_key", "quit key"),
        ("resize_schedule", "resize schedule"),
    ] {
        let left_value = path.split('.').fold(left_meta, |value, key| &value[key]);
        let right_value = path.split('.').fold(right_meta, |value, key| &value[key]);
        if left_value != right_value {
            bail!("capture metadata mismatch for {label}: left={left_value} right={right_value}");
        }
    }
    Ok(())
}

fn validate_capture_artifacts(meta: &Value, metadata_path: &Path) -> Result<()> {
    for artifact in ["cast", "raw", "settled_ansi", "grok_doctor"] {
        let path = meta
            .pointer(&format!("/artifacts/{artifact}"))
            .and_then(Value::as_str)
            .map(Path::new)
            .with_context(|| {
                format!(
                    "capture metadata missing artifacts.{artifact}: {}",
                    metadata_path.display()
                )
            })?;
        if !path.exists() {
            bail!(
                "capture metadata artifact does not exist (artifacts.{artifact}): {}",
                path.display()
            );
        }
    }
    Ok(())
}

fn validate_resize_artifact(meta: &Value, metadata_path: &Path) -> Result<()> {
    let Some(report_path) = meta
        .pointer("/artifacts/resize_report")
        .and_then(Value::as_str)
        .map(Path::new)
    else {
        bail!(
            "capture metadata resize report is missing: {}",
            metadata_path.display()
        );
    };
    if !report_path.exists() {
        bail!(
            "capture metadata resize report artifact does not exist: {}",
            report_path.display()
        );
    }
    let report: Value = serde_json::from_str(
        &std::fs::read_to_string(report_path)
            .with_context(|| format!("read resize report: {}", report_path.display()))?,
    )?;
    validate_resize_report(&report, meta["resize_schedule"].as_str().unwrap_or(""))
        .with_context(|| format!("resize report: {}", report_path.display()))
}

fn validate_resize_report(report: &Value, schedule: &str) -> Result<()> {
    if report.get("valid") != Some(&Value::Bool(true)) {
        bail!("resize report is not valid");
    }
    let expected = parse_resize_schedule(schedule)?;
    let observed = report
        .get("observed")
        .and_then(Value::as_array)
        .context("resize report observed array")?;
    if observed.len() != expected.len() {
        bail!(
            "expected {} observed resize events, got {}",
            expected.len(),
            observed.len()
        );
    }
    for ((expected_at, expected_geometry), actual) in expected.iter().zip(observed) {
        let actual_at = actual.get("at_ms").and_then(Value::as_u64);
        let actual_geometry = actual.get("geometry").and_then(Value::as_str);
        if actual_at != Some(*expected_at) || actual_geometry != Some(expected_geometry.as_str()) {
            bail!("observed resize does not match {expected_at}ms {expected_geometry}");
        }
    }
    Ok(())
}

fn parse_resize_schedule(schedule: &str) -> Result<Vec<(u64, String)>> {
    schedule
        .split(';')
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let mut parts = entry.split(',');
            let at_ms = parts
                .next()
                .context("resize entry timestamp")?
                .parse::<u64>()?;
            let cols = parts
                .next()
                .context("resize entry columns")?
                .parse::<u16>()?;
            let rows = parts.next().context("resize entry rows")?.parse::<u16>()?;
            anyhow::Ok((at_ms, format!("{cols},{rows}")))
        })
        .collect::<Result<Vec<_>>>()
}

fn validate_capture_metadata_shape(meta: &Value, path: &Path) -> Result<()> {
    validate_required_metadata_strings(meta, path)?;
    validate_terminal_dimensions(meta, path)
}

fn validate_required_metadata_strings(meta: &Value, path: &Path) -> Result<()> {
    let required_strings = [
        "captured_at",
        "repo_revision",
        "command",
        "grok_path",
        "grok_version",
        "capture_tools.tmux",
        "capture_tools.asciinema",
        "terminal.term",
        "terminal.colorterm",
        "artifacts.cast",
        "artifacts.raw",
        "artifacts.settled_ansi",
        "artifacts.grok_doctor",
        "artifacts.resize_report",
        "probe.prompt",
        "probe.quit_key",
    ];
    for dotted in required_strings {
        let value = dotted
            .split('.')
            .try_fold(meta, |value, key| value.get(key))
            .with_context(|| format!("capture metadata missing {dotted}: {}", path.display()))?;
        if !value.is_string() || value.as_str().is_some_and(str::is_empty) {
            bail!(
                "capture metadata field {dotted} must be a non-empty string: {}",
                path.display()
            );
        }
    }
    Ok(())
}

fn validate_terminal_dimensions(meta: &Value, path: &Path) -> Result<()> {
    for dotted in ["terminal.cols", "terminal.rows"] {
        let value = dotted
            .split('.')
            .try_fold(meta, |value, key| value.get(key))
            .with_context(|| format!("capture metadata missing {dotted}: {}", path.display()))?;
        if value.as_u64().is_none_or(|size| size == 0) {
            bail!(
                "capture metadata field {dotted} must be a positive integer: {}",
                path.display()
            );
        }
    }
    Ok(())
}

fn cells(parser: &vt100::Parser, rows: u16, cols: u16) -> Vec<Cell> {
    let screen = parser.screen();
    (0..rows)
        .flat_map(|row| {
            (0..cols).map(move |col| {
                let cell = screen.cell(row, col).expect("parser cell");
                Cell {
                    symbol: if cell.contents().is_empty() {
                        " ".into()
                    } else {
                        cell.contents()
                    },
                    width: if cell.is_wide_continuation() {
                        0
                    } else if cell.is_wide() {
                        2
                    } else {
                        1
                    },
                    fg: color_key(cell.fgcolor()),
                    bg: color_key(cell.bgcolor()),
                    bold: cell.bold(),
                    italic: cell.italic(),
                    underline: cell.underline(),
                    inverse: cell.inverse(),
                }
            })
        })
        .collect()
}

fn replay_frames(path: &Path, marker: Option<&str>) -> Result<FrameReplay> {
    let content = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let has_alternate_screen = content.contains("\u{1b}[?1049h");
    let mut lines = content.lines().peekable();
    let header: Value = serde_json::from_str(lines.next().context("cast header")?)?;
    let (cols, rows) = dimensions(&header)?;
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let (marker_text, marker_occurrence) = parse_marker(marker);
    let marker_texts = marker_text.split("&&").collect::<Vec<_>>();
    let started = marker.is_none();
    let config = ReplayConfig {
        has_alternate_screen,
        rows,
        cols,
        marker_texts: &marker_texts,
        marker_occurrence,
    };
    let (frames, started) = collect_replay_frames(&mut lines, &mut parser, config, started)?;
    ensure_marker_found(marker, started, marker_occurrence, path)?;
    Ok(((cols, rows), frames))
}

struct ReplayConfig<'a> {
    has_alternate_screen: bool,
    rows: u16,
    cols: u16,
    marker_texts: &'a [&'a str],
    marker_occurrence: usize,
}

fn collect_replay_frames(
    lines: &mut std::iter::Peekable<std::str::Lines<'_>>,
    parser: &mut vt100::Parser,
    config: ReplayConfig<'_>,
    mut started: bool,
) -> Result<(Vec<Vec<Cell>>, bool)> {
    let (mut frames, mut previous) = (Vec::new(), None);
    let (mut seen_markers, mut marker_visible, mut entered_alternate_screen) = (0, false, false);
    while let Some(line) = lines.next() {
        let event: Value = serde_json::from_str(line)?;
        if event[1].as_str() != Some("o") {
            continue;
        }
        let output = event[2].as_str().context("output payload")?;
        let Some(exited_alternate_screen) = prepare_cast_output(
            lines,
            parser,
            output,
            config.has_alternate_screen,
            &mut entered_alternate_screen,
        ) else {
            continue;
        };
        advance_marker(
            parser,
            &config,
            &mut started,
            &mut marker_visible,
            &mut seen_markers,
        );
        if !started {
            continue;
        }
        append_changed_frame(parser, config.rows, config.cols, &mut previous, &mut frames);
        if exited_alternate_screen {
            break;
        }
    }
    Ok((frames, started))
}

fn prepare_cast_output(
    lines: &mut std::iter::Peekable<std::str::Lines<'_>>,
    parser: &mut vt100::Parser,
    output: &str,
    has_alternate_screen: bool,
    entered_alternate_screen: &mut bool,
) -> Option<bool> {
    if should_stop_before_exit(output, lines) {
        return None;
    }
    apply_cast_output(
        parser,
        output,
        has_alternate_screen,
        entered_alternate_screen,
    )
}

fn advance_marker(
    parser: &vt100::Parser,
    config: &ReplayConfig<'_>,
    started: &mut bool,
    marker_visible: &mut bool,
    seen_markers: &mut usize,
) {
    if *started {
        return;
    }
    *started = marker_is_reached(
        parser,
        config.marker_texts,
        marker_visible,
        seen_markers,
        config.marker_occurrence,
    );
}

fn should_stop_before_exit(
    output: &str,
    lines: &mut std::iter::Peekable<std::str::Lines<'_>>,
) -> bool {
    output.contains("\u{1b}[2J") && lines.peek().is_some_and(|next| next.contains("?1049l"))
}

fn ensure_marker_found(
    marker: Option<&str>,
    started: bool,
    occurrence: usize,
    path: &Path,
) -> Result<()> {
    if marker.is_some() && !started {
        bail!(
            "phase marker {:?} occurrence {occurrence} was not found in {}",
            marker,
            path.display()
        );
    }
    Ok(())
}

fn apply_cast_output(
    parser: &mut vt100::Parser,
    output: &str,
    has_alternate_screen: bool,
    entered_alternate_screen: &mut bool,
) -> Option<bool> {
    let exited = output.contains("\u{1b}[?1049l");
    let output = output
        .split_once("\u{1b}[?1049l")
        .map_or(output, |(before_exit, _)| before_exit);
    if output.contains("\u{1b}[?1049h") {
        *entered_alternate_screen = true;
    } else if has_alternate_screen && !*entered_alternate_screen {
        return None;
    }
    parser.process(strip_private_modes(&output.replace("\u{1b}[?1049h", "")).as_bytes());
    Some(exited)
}

#[path = "cast_compare_support/replay.rs"]
mod replay_helpers;
pub(crate) use replay_helpers::*;

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "the CLI keeps replay, diagnostics, and exact exit semantics together"
)]
#[path = "../cast_compare_main.rs"]
mod cast_compare_main;
fn main() -> anyhow::Result<()> {
    cast_compare_main::run()
}
#[cfg(test)]
#[path = "../cast_compare_tests.rs"]
mod tests;
