//! Tests for fuzzy matching using `sublime_fuzzy`

/// Score a fuzzy match between `query` and `candidate`.
fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    sublime_fuzzy::best_match(query, candidate).map(|m| m.score() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_scores_highest() {
        let score = fuzzy_score("hello", "hello").unwrap();
        let partial = fuzzy_score("hel", "hello").unwrap();
        assert!(
            score > partial,
            "Exact match should score higher than partial"
        );
    }

    #[test]
    fn partial_match_in_order() {
        assert!(fuzzy_score("abc", "aabbcc").is_some());
    }

    #[test]
    fn missing_char_returns_none() {
        assert!(fuzzy_score("xyz", "abc").is_none());
    }

    #[test]
    fn case_insensitive_match() {
        assert!(fuzzy_score("HEL", "hello").is_some());
    }

    #[test]
    fn shorter_query_matches_longer() {
        let s = fuzzy_score("src", "source.rs").unwrap();
        let l = fuzzy_score("srcmain", "source.rs");
        assert!(
            l.is_none() || l.unwrap() < s,
            "Shorter match should score higher or longer should not match"
        );
    }

    #[test]
    fn word_boundary_bonus() {
        let with_boundary = fuzzy_score("mr", "main.rs").unwrap();
        let without = fuzzy_score("mr", "mar").unwrap();
        assert!(
            with_boundary > without,
            "Match after dot boundary should score higher"
        );
    }

    #[test]
    fn empty_query_returns_none() {
        // sublime_fuzzy returns None for empty query (no characters to match)
        assert!(fuzzy_score("", "anything").is_none());
    }

    #[test]
    fn fuzzy_filters_candidates() {
        let candidates = ["main.rs", "lib.rs", "README.md", "Cargo.toml"];
        let query = "mr";
        let mut results: Vec<_> = candidates
            .iter()
            .filter_map(|c| fuzzy_score(query, c).map(|s| (c, s)))
            .collect();
        results.sort_by_key(|b| std::cmp::Reverse(b.1));
        assert_eq!(results[0].0, &"main.rs", "Should match main.rs best");
    }

    #[test]
    fn fuzzy_score_exact_match_beats_partial() {
        let exact = fuzzy_score("hello", "hello").unwrap();
        let partial = fuzzy_score("hel", "hello").unwrap();
        assert!(exact > partial, "Exact match should beat partial match");
    }

    #[test]
    fn fuzzy_score_start_bonus() {
        let start = fuzzy_score("he", "hello").unwrap();
        let middle = fuzzy_score("he", "ache").unwrap();
        assert!(start > middle, "Start-of-word match should score higher");
    }

    #[test]
    fn panel_filter_and_at_ref_agree_on_order() {
        use crate::dialog::score::match_score;
        let candidates = ["main.rs", "lib.rs", "README.md"];
        let query = "mr";

        let mut at_ref: Vec<_> = candidates
            .iter()
            .filter_map(|c| fuzzy_score(query, c).map(|s| (c, s)))
            .collect();
        at_ref.sort_by_key(|b| std::cmp::Reverse(b.1));

        let mut panel: Vec<_> = candidates
            .iter()
            .filter_map(|c| match_score(c, query).map(|s| (c, s)))
            .collect();
        panel.sort_by_key(|b| std::cmp::Reverse(b.1));

        assert_eq!(at_ref[0].0, panel[0].0, "Top result should agree");
    }
}
