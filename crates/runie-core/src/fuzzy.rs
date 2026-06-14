//! Fuzzy string matching for @-ref completions and panel filtering.

/// Score a fuzzy match between `query` and `candidate`.
/// Returns `Some(score)` if every query char appears in order in the candidate,
/// `None` otherwise. Higher scores are better.
pub fn score(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let query_lower = query.to_lowercase();
    let cand_lower = candidate.to_lowercase();
    let mut score = 0i32;
    let mut ci = 0usize;

    for (qi, qc) in query_lower.chars().enumerate() {
        let (new_ci, step) = score_query_char(qc, qi, ci, &cand_lower)?;
        ci = new_ci;
        score += step;
    }
    score -= (cand_lower.len() - query_lower.len()) as i32;
    Some(score)
}

fn score_query_char(qc: char, qi: usize, ci: usize, cand_lower: &str) -> Option<(usize, i32)> {
    let pos = cand_lower[ci..].find(qc)?;
    let abs_pos = ci + pos;
    let mut step = 10i32;
    if abs_pos == 0 {
        step += 5;
    }
    step += word_boundary_bonus(cand_lower, abs_pos);
    if qi > 0 {
        step -= pos as i32;
    }
    Some((abs_pos + 1, step))
}

fn word_boundary_bonus(cand_lower: &str, abs_pos: usize) -> i32 {
    if abs_pos == 0 {
        return 0;
    }
    let prev = cand_lower.as_bytes()[abs_pos - 1];
    if matches!(prev, b'.' | b'/' | b'-' | b'_') {
        10
    } else {
        0
    }
}

/// Backward-compatible alias for `score`.
pub fn fuzzy_match(query: &str, candidate: &str) -> Option<i32> {
    score(query, candidate)
}

/// Filter and rank candidates by fuzzy match score.
/// Returns top `limit` matches sorted by score (highest first).
pub fn fuzzy_filter<'a>(query: &str, candidates: &[&'a str], limit: usize) -> Vec<&'a str> {
    let mut scored: Vec<(i32, &'a str)> = candidates
        .iter()
        .filter_map(|c| fuzzy_match(query, c).map(|s| (s, *c)))
        .collect();
    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    scored.into_iter().take(limit).map(|(_, c)| c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        assert_eq!(fuzzy_match("hello", "hello"), Some(55));
    }

    #[test]
    fn no_match() {
        assert_eq!(fuzzy_match("xyz", "abc"), None);
    }

    #[test]
    fn filter_ranking() {
        let candidates = vec!["main.rs", "lib.rs", "README.md"];
        let results = fuzzy_filter("mr", &candidates, 10);
        assert_eq!(results[0], "main.rs");
    }
}
