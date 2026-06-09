//! Tests for fuzzy file matching

#[cfg(test)]
mod tests {
    use crate::fuzzy::fuzzy_match;

    #[test]
    fn exact_match_scores_highest() {
        let score = fuzzy_match("hello", "hello").unwrap();
        let partial = fuzzy_match("hel", "hello").unwrap();
        assert!(score > partial, "Exact match should score higher than partial");
    }

    #[test]
    fn partial_match_in_order() {
        assert!(fuzzy_match("abc", "aabbcc").is_some());
    }

    #[test]
    fn missing_char_returns_none() {
        assert!(fuzzy_match("xyz", "abc").is_none());
    }

    #[test]
    fn case_insensitive_match() {
        assert!(fuzzy_match("HEL", "hello").is_some());
    }

    #[test]
    fn shorter_query_matches_longer() {
        let s = fuzzy_match("src", "source.rs").unwrap();
        let l = fuzzy_match("srcmain", "source.rs");
        assert!(l.is_none() || l.unwrap() < s, "Shorter match should score higher or longer should not match");
    }

    #[test]
    fn word_boundary_bonus() {
        let with_boundary = fuzzy_match("mr", "main.rs").unwrap();
        let without = fuzzy_match("mr", "mar").unwrap();
        assert!(with_boundary > without, "Match after dot boundary should score higher");
    }

    #[test]
    fn empty_query_matches_everything() {
        let score = fuzzy_match("", "anything").unwrap();
        assert_eq!(score, 0);
    }

    #[test]
    fn fuzzy_filters_candidates() {
        let candidates = ["main.rs", "lib.rs", "README.md", "Cargo.toml"];
        let query = "mr";
        let mut results: Vec<_> = candidates.iter()
            .filter_map(|c| fuzzy_match(query, c).map(|s| (c, s)))
            .collect();
        results.sort_by_key(|b| std::cmp::Reverse(b.1));
        assert_eq!(results[0].0, &"main.rs", "Should match main.rs best");
    }
}
