//! Unified diff generation for file edits

/// Represents a line in a unified diff
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLine {
    /// Context line (unchanged)
    Context(String),
    /// Added line (starts with +)
    Added(String),
    /// Removed line (starts with -)
    Removed(String),
    /// File header
    Header(String),
    /// Hunk header (e.g., @@ -1,5 +1,7 @@)
    HunkHeader(String),
}

/// Represents a hunk in a unified diff
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// Represents a complete unified diff
#[derive(Debug, Clone)]
pub struct UnifiedDiff {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<DiffHunk>,
}

/// Generates a unified diff between old and new content
pub fn generate_unified_diff(old_content: &str, new_content: &str) -> UnifiedDiff {
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();
    
    // Simple LCS-based diff algorithm
    let lcs = longest_common_subsequence(&old_lines, &new_lines);
    
    // If LCS contains all lines, content is identical - return empty diff
    if lcs.len() == old_lines.len() && old_lines.len() == new_lines.len() {
        return UnifiedDiff {
            old_path: "a".to_string(),
            new_path: "b".to_string(),
            hunks: Vec::new(),
        };
    }
    
    let hunks = build_hunks(&old_lines, &new_lines, &lcs);
    
    UnifiedDiff {
        old_path: "a".to_string(), // Placeholder, will be set by caller
        new_path: "b".to_string(),
        hunks,
    }
}

fn longest_common_subsequence<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(usize, usize)> {
    let m = old.len();
    let n = new.len();
    
    // Build DP table
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    
    for i in 1..=m {
        for j in 1..=n {
            if old[i - 1] == new[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    
    // Backtrack to find LCS
    let mut lcs = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 && j > 0 {
        if old[i - 1] == new[j - 1] {
            lcs.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    lcs.reverse();
    lcs
}

fn build_hunks(old_lines: &[&str], new_lines: &[&str], lcs: &[(usize, usize)]) -> Vec<DiffHunk> {
    if lcs.is_empty() {
        return vec![build_complete_replacement_hunk(old_lines, new_lines)];
    }
    
    let mut hunks = Vec::new();
    let mut old_idx = 0;
    let mut new_idx = 0;
    
    for (lcs_old, lcs_new) in lcs {
        if *lcs_old > old_idx || *lcs_new > new_idx {
            let hunk = collect_changes_between(old_lines, new_lines, *lcs_old, *lcs_new, &mut old_idx, &mut new_idx);
            if let Some(h) = hunk {
                hunks.push(h);
            }
        } else {
            old_idx = *lcs_old;
            new_idx = *lcs_new;
        }
    }
    
    // Handle trailing changes
    if old_idx < old_lines.len() || new_idx < new_lines.len() {
        let hunk = build_trailing_hunk(old_lines, new_lines, &mut old_idx, &mut new_idx);
        if let Some(h) = hunk {
            hunks.push(h);
        }
    }
    
    hunks
}

fn build_complete_replacement_hunk(old_lines: &[&str], new_lines: &[&str]) -> DiffHunk {
    let mut lines = Vec::new();
    for line in old_lines {
        lines.push(DiffLine::Removed((*line).to_string()));
    }
    for line in new_lines {
        lines.push(DiffLine::Added((*line).to_string()));
    }
    DiffHunk {
        header: format!("@@ -1,{} +1,{} @@", old_lines.len(), new_lines.len()),
        lines,
    }
}

fn count_removed(lines: &[DiffLine]) -> usize {
    lines.iter().filter(|l| matches!(l, DiffLine::Removed(_))).count()
}

fn count_added(lines: &[DiffLine]) -> usize {
    lines.iter().filter(|l| matches!(l, DiffLine::Added(_))).count()
}

fn collect_changes_between(
    old_lines: &[&str],
    new_lines: &[&str],
    lcs_old: usize,
    lcs_new: usize,
    old_idx: &mut usize,
    new_idx: &mut usize,
) -> Option<DiffHunk> {
    let mut hunk_lines = Vec::new();
    
    while *old_idx < lcs_old {
        hunk_lines.push(DiffLine::Removed(old_lines[*old_idx].to_string()));
        *old_idx += 1;
    }
    while *new_idx < lcs_new {
        hunk_lines.push(DiffLine::Added(new_lines[*new_idx].to_string()));
        *new_idx += 1;
    }
    
    if hunk_lines.is_empty() {
        return None;
    }
    
    let removed = count_removed(&hunk_lines);
    let added = count_added(&hunk_lines);
    let start_old = (*old_idx - removed).max(1);
    let start_new = (*new_idx - added).max(1);
    
    Some(DiffHunk {
        header: format!("@@ -{},{} +{},{} @@", start_old, removed, start_new, added),
        lines: hunk_lines,
    })
}

fn build_trailing_hunk(
    old_lines: &[&str],
    new_lines: &[&str],
    old_idx: &mut usize,
    new_idx: &mut usize,
) -> Option<DiffHunk> {
    let mut hunk_lines = Vec::new();
    
    while *old_idx < old_lines.len() {
        hunk_lines.push(DiffLine::Removed(old_lines[*old_idx].to_string()));
        *old_idx += 1;
    }
    while *new_idx < new_lines.len() {
        hunk_lines.push(DiffLine::Added(new_lines[*new_idx].to_string()));
        *new_idx += 1;
    }
    
    if hunk_lines.is_empty() {
        return None;
    }
    
    let removed = count_removed(&hunk_lines);
    let added = count_added(&hunk_lines);
    let start_old = (*old_idx - removed).max(1);
    let start_new = (*new_idx - added).max(1);
    
    Some(DiffHunk {
        header: format!("@@ -{},{} +{},{} @@", start_old, removed, start_new, added),
        lines: hunk_lines,
    })
}

/// Generate a preview of an edit without applying it.
pub fn preview_edit(path: &std::path::Path, old: &str, new: &str) -> anyhow::Result<runie_core::EditPreview> {
    let original = std::fs::read_to_string(path)?;
    let proposed = original.replacen(old, new, 1);
    let diff = generate_unified_diff(&original, &proposed);
    let diff_str = render_diff_to_string(&diff, &path.to_string_lossy());
    Ok(runie_core::EditPreview::new(
        path.to_path_buf(),
        original,
        proposed,
        diff_str,
    ))
}

/// Renders a unified diff to string format
pub fn render_diff_to_string(diff: &UnifiedDiff, path: &str) -> String {
    let mut output = Vec::new();
    
    output.push(format!("--- {}", path));
    output.push(format!("+++ {}", path));
    
    for hunk in &diff.hunks {
        output.push(hunk.header.clone());
        for line in &hunk.lines {
            match line {
                DiffLine::Context(s) => output.push(format!(" {}", s)),
                DiffLine::Added(s) => output.push(format!("+{}", s)),
                DiffLine::Removed(s) => output.push(format!("-{}", s)),
                DiffLine::Header(s) => output.push(s.clone()),
                DiffLine::HunkHeader(s) => output.push(s.clone()),
            }
        }
    }
    
    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_changes_empty_diff() {
        let content = "line1\nline2\nline3";
        let diff = generate_unified_diff(content, content);
        // When content is identical, LCS = all lines, so hunks may be empty
        assert!(diff.hunks.is_empty() || diff.hunks.iter().all(|h| h.lines.iter().all(|l| matches!(l, DiffLine::Context(_)))));
    }

    #[test]
    fn single_line_addition() {
        let old = "line1\nline2";
        let new = "line1\nline2\nline3";
        let diff = generate_unified_diff(old, new);
        assert!(!diff.hunks.is_empty());
        
        let has_added = diff.hunks.iter()
            .flat_map(|h| &h.lines)
            .any(|l| matches!(l, DiffLine::Added(s) if s == "line3"));
        assert!(has_added, "Should have added line3");
    }

    #[test]
    fn single_line_removal() {
        let old = "line1\nline2\nline3";
        let new = "line1\nline3";
        let diff = generate_unified_diff(old, new);
        assert!(!diff.hunks.is_empty());
        
        let has_removed = diff.hunks.iter()
            .flat_map(|h| &h.lines)
            .any(|l| matches!(l, DiffLine::Removed(s) if s == "line2"));
        assert!(has_removed, "Should have removed line2");
    }

    #[test]
    fn single_line_modification() {
        let old = "line1\nold_line\nline3";
        let new = "line1\nnew_line\nline3";
        let diff = generate_unified_diff(old, new);
        assert!(!diff.hunks.is_empty());
        
        let has_removed = diff.hunks.iter()
            .flat_map(|h| &h.lines)
            .any(|l| matches!(l, DiffLine::Removed(s) if s == "old_line"));
        let has_added = diff.hunks.iter()
            .flat_map(|h| &h.lines)
            .any(|l| matches!(l, DiffLine::Added(s) if s == "new_line"));
        
        assert!(has_removed, "Should have removed old_line");
        assert!(has_added, "Should have added new_line");
    }

    #[test]
    fn render_diff_to_string_format() {
        let old = "line1";
        let new = "line2";
        let diff = generate_unified_diff(old, new);
        let output = render_diff_to_string(&diff, "test.txt");
        
        assert!(output.contains("--- test.txt"));
        assert!(output.contains("+++ test.txt"));
        assert!(output.contains("-line1"));
        assert!(output.contains("+line2"));
    }

    #[test]
    fn empty_old_content() {
        let old = "";
        let new = "new content";
        let diff = generate_unified_diff(old, new);
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn empty_new_content() {
        let old = "old content";
        let new = "";
        let diff = generate_unified_diff(old, new);
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn multi_line_addition() {
        let old = "line1";
        let new = "line1\nline2\nline3";
        let diff = generate_unified_diff(old, new);
        let output = render_diff_to_string(&diff, "test.txt");
        
        assert!(output.contains("+line2"));
        assert!(output.contains("+line3"));
    }

    #[test]
    fn preview_generates_diff() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();

        let preview = preview_edit(&path, "world", "universe").unwrap();
        assert!(preview.diff.contains("---"), "diff should have file header");
        assert!(preview.diff.contains("+++"), "diff should have file header");
        assert!(preview.diff.contains("-hello world"), "diff should show removed line");
        assert!(preview.diff.contains("+hello universe"), "diff should show added line");
        assert_eq!(preview.original, "hello world");
        assert_eq!(preview.proposed, "hello universe");
    }

    #[test]
    fn preview_shows_line_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "line1\nline2\nline3").unwrap();

        let preview = preview_edit(&path, "line2", "modified").unwrap();
        assert!(preview.diff.contains("@@"), "diff should have hunk header with line numbers");
    }
}
