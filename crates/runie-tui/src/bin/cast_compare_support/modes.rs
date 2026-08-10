use super::*;

pub fn run_frames(left: &Path, right: &Path, phase_marker: Option<&str>) -> Result<()> {
    let (left_geometry, left_frames) = replay_frames(left, phase_marker)?;
    let (right_geometry, right_frames) = replay_frames(right, phase_marker)?;
    ensure_geometry(left_geometry, right_geometry)?;
    let (exact, payload) = compare_frames(&left_frames, &right_frames, left_geometry.0);
    println!("{}", serde_json::to_string(&payload)?);
    exact
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("indexed cast frames differ"))
}

fn ensure_geometry(left: (u16, u16), right: (u16, u16)) -> Result<()> {
    if left == right {
        return Ok(());
    }
    bail!(
        "cast geometries differ: left {}x{}, right {}x{}",
        left.0,
        left.1,
        right.0,
        right.1
    )
}

fn compare_frames(left: &[Vec<Cell>], right: &[Vec<Cell>], cols: u16) -> (bool, Value) {
    let compared = left.len().min(right.len());
    let first = (0..compared).find(|&index| left[index] != right[index]);
    let cell =
        first.and_then(|frame| first_cell_difference(&left[frame], &right[frame], frame, cols));
    let counts = frame_cell_difference_counts(left, right);
    let common = ordered_common_frame_count(left, right);
    let exact = left.len() == right.len() && first.is_none();
    let payload = serde_json::json!({"left_frames":left.len(),"right_frames":right.len(),"compared_frames":compared,"ordered_common_frames":common,"left_unmatched_frames":left.len().saturating_sub(common),"right_unmatched_frames":right.len().saturating_sub(common),"different_frames":counts.iter().filter(|count| **count > 0).count() + left.len().abs_diff(right.len()),"frame_cell_differences":counts,"first_difference":first.map(|index| index + 1),"first_cell_difference":cell,"exact":exact});
    (exact, payload)
}

fn first_cell_difference(left: &[Cell], right: &[Cell], frame: usize, cols: u16) -> Option<Value> {
    left.iter().zip(right).position(|(left, right)| left != right).map(|cell| serde_json::json!({"frame":frame + 1,"x":cell % cols as usize,"y":cell / cols as usize,"left":left[cell],"right":right[cell]}))
}

#[derive(Default)]
struct CellDiffReport {
    glyphs: usize,
    attributes: usize,
    widths: usize,
    colors: usize,
    styles: usize,
    other_attributes: usize,
    row_differences: Vec<usize>,
    coordinates: Vec<(usize, usize)>,
    attribute_coordinates: Vec<Value>,
}

fn diff_cells(left: &[Cell], right: &[Cell], cols: u16, rows: u16) -> CellDiffReport {
    left.iter().zip(right).enumerate().fold(
        CellDiffReport {
            row_differences: vec![0; rows as usize],
            ..Default::default()
        },
        |mut report, (index, (left, right))| {
            record_cell(&mut report, index, left, right, cols);
            report
        },
    )
}

fn record_cell(report: &mut CellDiffReport, index: usize, left: &Cell, right: &Cell, cols: u16) {
    let row = index / cols as usize;
    if left.symbol == right.symbol {
        record_attributes(report, index, left, right, row, cols);
        return;
    }
    report.glyphs += 1;
    report.row_differences[row] += 1;
    if report.coordinates.len() < 20 {
        report.coordinates.push((index % cols as usize, row));
    }
}

fn record_attributes(
    report: &mut CellDiffReport,
    index: usize,
    left: &Cell,
    right: &Cell,
    row: usize,
    cols: u16,
) {
    if left == right {
        return;
    }
    report.attributes += 1;
    report.widths += usize::from(left.width != right.width);
    report.colors += usize::from(left.fg != right.fg || left.bg != right.bg);
    let (styles, other) = attribute_style_counts(left, right);
    report.styles += styles;
    report.other_attributes += other;
    report.row_differences[row] += 1;
    if report.attribute_coordinates.len() < 20 {
        report.attribute_coordinates.push(
            serde_json::json!({"x": index % cols as usize, "y": row, "left": left, "right": right}),
        );
    }
}

fn attribute_style_counts(left: &Cell, right: &Cell) -> (usize, usize) {
    let styles = left.bold != right.bold
        || left.italic != right.italic
        || left.underline != right.underline
        || left.inverse != right.inverse;
    let other = left.width == right.width && left.fg == right.fg && left.bg == right.bg && !styles;
    (usize::from(styles), usize::from(other))
}

fn print_row_differences(rows: &[usize], left: &[String], right: &[String]) {
    for (row, count) in rows.iter().enumerate().filter(|(_, count)| **count > 0) {
        println!(
            "row {} ({} diffs)\n  left:  {:?}\n  right: {:?}",
            row + 1,
            count,
            left.get(row).map(String::as_str).unwrap_or_default(),
            right.get(row).map(String::as_str).unwrap_or_default()
        );
    }
}

pub fn run_cells(left: &str, right: &str, dump: bool) -> Result<()> {
    let (left_geometry, left_cells, left_lines) = replay(Path::new(&left))?;
    let (right_geometry, right_cells, right_lines) = replay(Path::new(&right))?;
    if dump {
        return dump_cells(
            left,
            right,
            left_geometry,
            right_geometry,
            left_cells,
            right_cells,
        );
    }
    validate_cell_geometry(left_geometry, right_geometry)?;
    let report = diff_cells(
        &left_cells,
        &right_cells,
        left_geometry.0,
        left_geometry.1.max(right_geometry.1),
    );
    finish_cells(
        report,
        left_geometry,
        right_geometry,
        left_cells.len(),
        right_cells.len(),
        &left_lines,
        &right_lines,
    )
}

fn dump_cells(
    left: &str,
    right: &str,
    left_geometry: (u16, u16),
    right_geometry: (u16, u16),
    left_cells: Vec<Cell>,
    right_cells: Vec<Cell>,
) -> Result<()> {
    let payload = serde_json::json!({"left":{"path":left,"cols":left_geometry.0,"rows":left_geometry.1,"cells":left_cells},"right":{"path":right,"cols":right_geometry.0,"rows":right_geometry.1,"cells":right_cells}});
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn validate_cell_geometry(left: (u16, u16), right: (u16, u16)) -> Result<()> {
    if left == right {
        return Ok(());
    }
    println!("{{\"exact\":false,\"error\":\"geometry_mismatch\",\"left\":{{\"cols\":{},\"rows\":{}}},\"right\":{{\"cols\":{},\"rows\":{}}}}}", left.0, left.1, right.0, right.1);
    bail!(
        "cast geometries differ: left {}x{}, right {}x{}",
        left.0,
        left.1,
        right.0,
        right.1
    )
}

fn finish_cells(
    report: CellDiffReport,
    left: (u16, u16),
    right: (u16, u16),
    left_len: usize,
    right_len: usize,
    left_lines: &[String],
    right_lines: &[String],
) -> Result<()> {
    let different = report.glyphs + report.attributes + left_len.abs_diff(right_len);
    let hotspots = report
        .row_differences
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(row, count)| format!("{}:{count}", row + 1))
        .collect::<Vec<_>>();
    let payload = serde_json::json!({"left":{"cols":left.0,"rows":left.1},"right":{"cols":right.0,"rows":right.1},"compared_cells":left_len.min(right_len),"different_cells":different,"different_glyphs":report.glyphs,"different_attributes":report.attributes,"attribute_breakdown":{"width":report.widths,"colors":report.colors,"styles":report.styles,"other":report.other_attributes},"row_hotspots":hotspots,"glyph_coordinates":report.coordinates,"attribute_coordinates":report.attribute_coordinates,"exact":different == 0});
    println!("{}", serde_json::to_string(&payload)?);
    print_row_differences(&report.row_differences, left_lines, right_lines);
    different
        .eq(&0)
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("casts differ"))
}
