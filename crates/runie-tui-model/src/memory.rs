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

/// Project the Grok memory protocol into renderer-neutral transcript rows.
///
/// Keeping this formatting beside the parser makes live event projection and
/// YAML replay consume one semantic contract; terminal styling remains in the
/// renderer.
pub fn memory_display_lines(output: &str) -> Vec<String> {
    let results = parse_memory_results(output);
    if results.is_empty() {
        return output
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect();
    }
    results
        .iter()
        .enumerate()
        .flat_map(|(index, result)| {
            let path = display_memory_path(&result.path);
            let location = if result.start_line == 0 && result.end_line == 0 {
                path
            } else {
                format!("{}:{}-{}", path, result.start_line, result.end_line)
            };
            std::iter::once(format!(
                "  {}. {}  (score: {:.2}, {})",
                index + 1,
                location,
                result.score,
                result.source
            ))
            .chain(
                result
                    .snippet
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .take(3)
                    .map(|line| format!("    {}", line.trim())),
            )
        })
        .collect()
}

/// Grok's memory card does not expose the installation-specific memory root.
/// Keep the projection deterministic across machines by retaining the path
/// below a `/memory/` segment and falling back to the final path component.
fn display_memory_path(path: &str) -> String {
    if let Some((_, relative)) = path.rsplit_once("/memory/") {
        return relative.to_owned();
    }
    path.rsplit('/').next().unwrap_or(path).to_owned()
}

fn parse_section(section: &str) -> Option<MemoryResult> {
    let mut lines = section.lines();
    let header = lines.next()?;
    if !header.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let (score, source) = parse_memory_header(header);
    let body: Vec<&str> = lines.collect();
    let (path, start_line, end_line) = parse_memory_location(&body);
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

fn parse_memory_header(header: &str) -> (f64, String) {
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
    (score, source)
}

fn parse_memory_location(body: &[&str]) -> (String, usize, usize) {
    let mut location = (String::new(), 0, 0);
    for line in body {
        let Some(rest) = line.strip_prefix("**File:** ") else {
            continue;
        };
        let Some((file, range)) = rest.split_once(" (lines ") else {
            location.0 = rest.to_owned();
            continue;
        };
        location.0 = file.to_owned();
        if let Some((start, end)) = range.trim_end_matches(')').split_once('-') {
            location.1 = start.parse().unwrap_or(0);
            location.2 = end.parse().unwrap_or(0);
        }
    }
    location
}

#[cfg(test)]
mod tests {
    use super::{memory_display_lines, parse_memory_results};

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

    #[test]
    fn projects_structured_results_into_shared_transcript_rows() {
        let rows = memory_display_lines(
            "### Result 1 (score: 0.72, source: global)\n**File:** /var/lib/grok/memory/memory.md (lines 1-2)\n```\nalpha\n```",
        );
        assert_eq!(
            rows,
            ["  1. memory.md:1-2  (score: 0.72, global)", "    alpha"]
        );
    }

    #[test]
    fn memory_rows_hide_installation_specific_roots() {
        let rows = memory_display_lines(
            "### Result 1 (score: 0.72, source: global)\n**File:** /memory/MEMORY.md (lines 1-2)\n```\nalpha\n```\n### Result 2 (score: 0.42, source: session)\n**File:** /tmp/session.md (lines 3-4)\n```\nbeta\n```",
        );
        assert_eq!(rows[0], "  1. MEMORY.md:1-2  (score: 0.72, global)");
        assert_eq!(rows[2], "  2. session.md:3-4  (score: 0.42, session)");
    }
}
