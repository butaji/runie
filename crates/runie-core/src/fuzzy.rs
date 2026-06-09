//! Fuzzy string matching for @-ref completions

/// Score a fuzzy match between query and candidate.
/// Returns Some(score) if query chars appear in order in candidate,
/// None otherwise. Higher scores are better.
pub fn fuzzy_match(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let query_lower = query.to_lowercase();
    let cand_lower = candidate.to_lowercase();
    let mut score = 0i32;
    let mut ci = 0usize;

    for (qi, qc) in query_lower.chars().enumerate() {
        if let Some(pos) = cand_lower[ci..].find(qc) {
            let abs_pos = ci + pos;
            ci = abs_pos + 1;
            score += 10;
            // Bonus for matching at start of string
            if abs_pos == 0 {
                score += 5;
            }
            // Bonus for matching after word boundary (. / - _)
            if abs_pos > 0 {
                let prev = cand_lower.as_bytes()[abs_pos - 1];
                if prev == b'.' || prev == b'/' || prev == b'-' || prev == b'_' {
                    score += 10;
                }
            }
            // Small penalty for distance from previous match
            if qi > 0 {
                score -= pos as i32;
            }
        } else {
            return None;
        }
    }
    // Penalty for length difference
    score -= (cand_lower.len() - query_lower.len()) as i32;
    Some(score)
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
