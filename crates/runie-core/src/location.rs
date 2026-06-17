//! Location parsing for file:line:col patterns.
//!
//! Re-exports [`fff_search::Location`] and provides helpers for parsing
//! file references like `@src/lib.rs:10:5` into structured location data.

pub use fff_search::Location;

/// Parse a `file:line:col` pattern into a path and optional location.
///
/// Returns `(path, location)` where `location` is `None` if no valid
/// location suffix was found.
///
/// # Examples
/// ```
/// use runie_core::location::{parse_location, Location};
/// assert_eq!(parse_location("src/lib.rs:10:5"), ("src/lib.rs", Some(Location::Position { line: 10, col: 5 })));
/// assert_eq!(parse_location("src/lib.rs:10"), ("src/lib.rs", Some(Location::Line(10))));
/// assert_eq!(parse_location("src/lib.rs"), ("src/lib.rs", None));
/// ```
pub fn parse_location(query: &str) -> (&str, Option<Location>) {
    // Parse a `file:line` or `file:line:col` pattern.
    // This mirrors the logic from fff_query_parser::location::parse_location.
    let (file_path, location_part) = match query.split_once(':') {
        Some((p, loc)) => (p, loc),
        None => return (query, None),
    };

    if location_part.contains('-') {
        let (path, loc) = parse_range(file_path, location_part);
        if loc.is_some() {
            return (path, loc);
        }
    }
    if location_part.contains(':') {
        let (path, loc) = parse_position(file_path, location_part);
        if loc.is_some() {
            return (path, loc);
        }
    }
    let (path, loc) = parse_line(file_path, location_part);
    if loc.is_some() {
        return (path, loc);
    }

    // Not a valid location — treat the whole thing as the path
    (query, None)
}

fn parse_range<'a>(file_path: &'a str, location_part: &str) -> (&'a str, Option<Location>) {
    let Some((start_part, end_part)) = location_part.split_once('-') else {
        return (file_path, None);
    };
    if start_part.contains(':') && end_part.contains(':') {
        let Some((start_line, start_col)) = parse_line_col(start_part) else {
            return (file_path, None);
        };
        let Some((end_line, end_col)) = parse_line_col(end_part) else {
            return (file_path, None);
        };
        return (
            file_path,
            Some(Location::Range {
                start: (start_line, start_col),
                end: (end_line, end_col),
            }),
        );
    }
    if start_part.contains(':') {
        let Some((line, start_col)) = parse_line_col(start_part) else {
            return (file_path, None);
        };
        let Some(end_col) = end_part.parse::<i32>().ok() else {
            return (file_path, None);
        };
        return (
            file_path,
            Some(Location::Range {
                start: (line, start_col),
                end: (line, end_col),
            }),
        );
    }
    (file_path, parse_line_range(start_part, end_part))
}

fn parse_line_range(start_part: &str, end_part: &str) -> Option<Location> {
    let start_line = start_part.parse::<i32>().ok()?;
    let end_line = end_part.parse::<i32>().ok()?;
    let end = end_line.max(start_line);
    Some(Location::Range {
        start: (start_line, 0),
        end: (end, 0),
    })
}

fn parse_line_col(part: &str) -> Option<(i32, i32)> {
    let (line_str, col_str) = part.split_once(':')?;
    Some((line_str.parse::<i32>().ok()?, col_str.parse::<i32>().ok()?))
}

fn parse_position<'a>(file_path: &'a str, location_part: &str) -> (&'a str, Option<Location>) {
    let Some((line, col)) = parse_line_col(location_part) else {
        return (file_path, None);
    };
    (file_path, Some(Location::Position { line, col }))
}

fn parse_line<'a>(file_path: &'a str, location_part: &str) -> (&'a str, Option<Location>) {
    let Ok(line) = location_part.parse::<i32>() else {
        return (file_path, None);
    };
    (file_path, Some(Location::Line(line)))
}

/// Returns the line number from a location, if present.
pub fn location_line(loc: &Location) -> i32 {
    match loc {
        Location::Line(l) => *l,
        Location::Position { line, .. } => *line,
        Location::Range { start, .. } => start.0,
    }
}

/// Returns the column number from a location, if present.
pub fn location_col(loc: &Location) -> Option<i32> {
    match loc {
        Location::Line(_) => None,
        Location::Position { col, .. } => Some(*col),
        Location::Range { start, .. } => Some(start.1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_extracts_line_and_column() {
        let (path, loc) = parse_location("src/lib.rs:10:5");
        assert_eq!(path, "src/lib.rs");
        assert!(matches!(loc, Some(Location::Position { line: 10, col: 5 })));
    }

    #[test]
    fn parser_handles_missing_column() {
        let (path, loc) = parse_location("src/lib.rs:10");
        assert_eq!(path, "src/lib.rs");
        assert!(matches!(loc, Some(Location::Line(10))));
    }

    #[test]
    fn parser_handles_no_location() {
        let (path, loc) = parse_location("src/lib.rs");
        assert_eq!(path, "src/lib.rs");
        assert!(loc.is_none());
    }

    #[test]
    fn parser_handles_line_range() {
        let (path, loc) = parse_location("src/lib.rs:10-20");
        assert_eq!(path, "src/lib.rs");
        assert!(matches!(
            loc,
            Some(Location::Range { start, end })
            if start == (10, 0) && end == (20, 0)
        ));
    }

    #[test]
    fn parser_handles_column_range() {
        let (path, loc) = parse_location("src/lib.rs:10:5-20");
        assert_eq!(path, "src/lib.rs");
        assert!(matches!(
            loc,
            Some(Location::Range { start, end })
            if start == (10, 5) && end == (10, 20)
        ));
    }

    #[test]
    fn location_line_extraction() {
        let (_, loc) = parse_location("src/lib.rs:42:7");
        let loc = loc.unwrap();
        assert_eq!(location_line(&loc), 42);
        assert_eq!(location_col(&loc), Some(7));
    }

    #[test]
    fn location_line_only() {
        let (_, loc) = parse_location("src/lib.rs:99");
        let loc = loc.unwrap();
        assert_eq!(location_line(&loc), 99);
        assert_eq!(location_col(&loc), None);
    }
}
