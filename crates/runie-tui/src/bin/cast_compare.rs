//! Compare two asciinema casts after replaying them through the same VT parser.

use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Cell {
    symbol: String,
    fg: String,
    bg: String,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
}

type Replay = ((u16, u16), Vec<Cell>, Vec<String>);

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
    if let Some(term) = header.get("term") {
        return Ok((
            term["cols"].as_u64().context("cast term.cols")? as u16,
            term["rows"].as_u64().context("cast term.rows")? as u16,
        ));
    }
    Ok((
        header["width"].as_u64().context("cast width")? as u16,
        header["height"].as_u64().context("cast height")? as u16,
    ))
}

#[allow(
    clippy::too_many_lines,
    reason = "cast replay keeps terminal normalization and cell extraction together"
)]
fn replay(path: &Path) -> Result<Replay> {
    let content = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let mut lines = content.lines();
    let header: Value = serde_json::from_str(lines.next().context("cast header")?)?;
    let (cols, rows) = dimensions(&header)?;
    let mut parser = vt100::Parser::new(rows, cols, 0);
    for line in lines {
        let event: Value = serde_json::from_str(line)?;
        if event[1].as_str() != Some("o") {
            continue;
        }
        let output = event[2]
            .as_str()
            .context("output payload")?
            // Normalize alternate-screen entry so casts recorded through
            // nested tmux/asciinema PTYs share one comparable virtual screen.
            .replace("\u{1b}[?1049h", "");
        // The shell frame after an alternate-screen exit is not part of the
        // TUI scenario. Keep the last application frame for parity checks.
        if output.contains("\u{1b}[?1049l") {
            break;
        }
        parser.process(strip_private_modes(&output).as_bytes());
    }
    let screen = parser.screen();
    let current_cells = (0..rows)
        .flat_map(|row| {
            (0..cols).map(move |col| {
                let cell = screen.cell(row, col).expect("parser cell");
                Cell {
                    symbol: if cell.contents().is_empty() {
                        " ".into()
                    } else {
                        cell.contents()
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
        .collect();
    let current_contents = screen.contents().lines().map(str::to_owned).collect();
    Ok(((cols, rows), current_cells, current_contents))
}

#[allow(
    clippy::too_many_lines,
    reason = "the CLI keeps replay, diagnostics, and exact exit semantics together"
)]
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let dump = args.first().is_some_and(|arg| arg == "--dump");
    if dump {
        args.remove(0);
    }
    let mut args = args.into_iter();
    let left = args.next().context(if dump {
        "usage: cast_compare --dump LEFT.cast RIGHT.cast"
    } else {
        "usage: cast_compare LEFT.cast RIGHT.cast"
    })?;
    let right = args
        .next()
        .context("usage: cast_compare LEFT.cast RIGHT.cast")?;
    if args.next().is_some() {
        bail!(
            "usage: cast_compare{} LEFT.cast RIGHT.cast",
            if dump { " --dump" } else { "" }
        );
    }
    let (left_geometry, left_cells, left_lines) = replay(Path::new(&left))?;
    let (right_geometry, right_cells, right_lines) = replay(Path::new(&right))?;
    if dump {
        let payload = serde_json::json!({
            "left": { "path": left, "cols": left_geometry.0, "rows": left_geometry.1, "cells": left_cells },
            "right": { "path": right, "cols": right_geometry.0, "rows": right_geometry.1, "cells": right_cells },
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }
    let geometry_equal = left_geometry == right_geometry;
    let compared = left_cells.len().min(right_cells.len());
    let mut glyphs = 0;
    let mut attributes = 0;
    let mut row_differences = vec![0usize; left_geometry.1.max(right_geometry.1) as usize];
    let mut coordinates = Vec::new();
    let mut attribute_coordinates = Vec::new();
    for (index, (left, right)) in left_cells.iter().zip(right_cells.iter()).enumerate() {
        if left.symbol != right.symbol {
            glyphs += 1;
            row_differences[index / left_geometry.0 as usize] += 1;
            if coordinates.len() < 20 {
                coordinates.push((
                    index % left_geometry.0 as usize,
                    index / left_geometry.0 as usize,
                ));
            }
        } else if left != right {
            attributes += 1;
            row_differences[index / left_geometry.0 as usize] += 1;
            if attribute_coordinates.len() < 20 {
                attribute_coordinates.push(serde_json::json!({
                    "x": index % left_geometry.0 as usize,
                    "y": index / left_geometry.0 as usize,
                    "left": left,
                    "right": right,
                }));
            }
        }
    }
    let different = glyphs + attributes + left_cells.len().abs_diff(right_cells.len());
    let hotspots: Vec<_> = row_differences
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(row, count)| format!("{}:{}", row + 1, count))
        .collect();
    println!(
        "{{\"left\":{{\"cols\":{},\"rows\":{}}},\"right\":{{\"cols\":{},\"rows\":{}}},\"compared_cells\":{},\"different_cells\":{},\"different_glyphs\":{},\"different_attributes\":{},\"row_hotspots\":[{}],\"glyph_coordinates\":{:?},\"attribute_coordinates\":{},\"exact\":{}}}",
        left_geometry.0,
        left_geometry.1,
        right_geometry.0,
        right_geometry.1,
        compared,
        different,
        glyphs,
        attributes,
        hotspots
            .iter()
            .map(|hotspot| format!("\"{hotspot}\""))
            .collect::<Vec<_>>()
            .join(","),
        coordinates,
        serde_json::to_string(&attribute_coordinates)?,
        geometry_equal && different == 0
    );
    for (row, count) in row_differences
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
    {
        println!(
            "row {} ({} diffs)\n  left:  {:?}\n  right: {:?}",
            row + 1,
            count,
            left_lines.get(row).map(String::as_str).unwrap_or_default(),
            right_lines.get(row).map(String::as_str).unwrap_or_default(),
        );
    }
    if geometry_equal && different == 0 {
        Ok(())
    } else {
        bail!("casts differ")
    }
}
