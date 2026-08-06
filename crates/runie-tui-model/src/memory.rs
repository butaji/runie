//! Pure parsing of Grok's structured memory-search result contract.

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryResult {
    pub score: f64,
    pub source: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub snippet: String,
}

/// Parse the markdown result protocol emitted by Grok's memory tool.
pub fn parse_memory_results(output: &str) -> Vec<MemoryResult> {
    output
        .split("### Result ")
        .filter_map(parse_section)
        .collect()
}

#[allow(
    clippy::too_many_lines,
    reason = "the Grok markdown protocol parser keeps one result grammar together"
)]
fn parse_section(section: &str) -> Option<MemoryResult> {
    let mut lines = section.lines();
    let header = lines.next()?;
    if !header.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let score = header
        .split_once("score: ")
        .and_then(|(_, rest)| rest.split(',').next())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0.0);
    let source = header
        .split_once("source: ")
        .and_then(|(_, rest)| rest.split(')').next())
        .unwrap_or_default()
        .to_owned();
    let body: Vec<&str> = lines.collect();
    let mut path = String::new();
    let mut start_line = 0;
    let mut end_line = 0;
    for line in &body {
        if let Some(rest) = line.strip_prefix("**File:** ") {
            if let Some((file, range)) = rest.split_once(" (lines ") {
                path = file.to_owned();
                if let Some((start, end)) = range.trim_end_matches(')').split_once('-') {
                    start_line = start.parse().unwrap_or(0);
                    end_line = end.parse().unwrap_or(0);
                }
            } else {
                path = rest.to_owned();
            }
        }
    }
    let joined = body.join("\n");
    let snippet = joined
        .split_once("```\n")
        .and_then(|(_, rest)| rest.split_once("\n```"))
        .map(|(snippet, _)| snippet.to_owned())
        .unwrap_or_default();
    if path.is_empty() && snippet.is_empty() {
        return None;
    }
    Some(MemoryResult {
        score,
        source,
        path,
        start_line,
        end_line,
        snippet,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_memory_results;

    #[test]
    fn parses_grok_memory_result_protocol() {
        let output = "### Result 1 (score: 0.72, source: global)\n**File:** /memory/MEMORY.md (lines 0-10)\n```\nalpha\nbeta\n```\n### Result 2 (score: 0.42, source: session)\n**File:** session.md (lines 4-7)\n```\ngamma\n```";
        let results = parse_memory_results(output);
        assert_eq!(results.len(), 2);
        assert!((results[0].score - 0.72).abs() < f64::EPSILON);
        assert_eq!(results[0].source, "global");
        assert_eq!(results[0].start_line, 0);
        assert_eq!(results[0].snippet, "alpha\nbeta");
    }

    #[test]
    fn ignores_no_result_message() {
        assert!(parse_memory_results("No memory results found").is_empty());
    }
}
