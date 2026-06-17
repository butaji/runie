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

    // Try line:col-col (column range on same line)
    if location_part.contains('-') {
        if let Some((start_part, end_part)) = location_part.split_once('-') {
            if start_part.contains(':') && end_part.contains(':') {
                // line:col-line:col
                if let Some((sl, sc)) = start_part.split_once(':') {
                    if let (Ok(start_line), Ok(start_col)) = (sl.parse::<i32>(), sc.parse::<i32>())
                    {
                        if let Some((el, ec)) = end_part.split_once(':') {
                            if let (Ok(end_line), Ok(end_col)) =
                                (el.parse::<i32>(), ec.parse::<i32>())
                            {
                                return (
                                    file_path,
                                    Some(Location::Range {
                                        start: (start_line, start_col),
                                        end: (end_line, end_col),
                                    }),
                                );
                            }
                        }
                    }
                }
            } else if start_part.contains(':') {
                // line:col-col (column range)
                if let Some((line_str, start_col_str)) = start_part.split_once(':') {
                    if let (Ok(line), Ok(start_col), Ok(end_col)) = (
                        line_str.parse::<i32>(),
                        start_col_str.parse::<i32>(),
                        end_part.parse::<i32>(),
                    ) {
                        return (
                            file_path,
                            Some(Location::Range {
                                start: (line, start_col),
                                end: (line, end_col),
                            }),
                        );
                    }
                }
            } else {
                // line-line (line range)
                if let (Ok(start_line), Ok(end_line)) =
                    (start_part.parse::<i32>(), end_part.parse::<i32>())
                {
                    let end = if end_line < start_line {
                        start_line
                    } else {
                        end_line
                    };
                    return (
                        file_path,
                        Some(Location::Range {
                            start: (start_line, 0),
                            end: (end, 0),
                        }),
                    );
                }
            }
        }
    }

    // Try line:col (position)
    if location_part.contains(':') {
        if let Some((line_str, col_str)) = location_part.split_once(':') {
            if let (Ok(line), Ok(col)) = (line_str.parse::<i32>(), col_str.parse::<i32>()) {
                return (file_path, Some(Location::Position { line, col }));
            }
        }
    }

    // Try just a line number
    if let Ok(line) = location_part.parse::<i32>() {
        return (file_path, Some(Location::Line(line)));
    }

    // Not a valid location — treat the whole thing as the path
    (query, None)
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
