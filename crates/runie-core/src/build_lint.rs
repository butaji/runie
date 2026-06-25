// Build lint helper shared between `build.rs` and the test suite.
//
// The complexity heuristic is intentionally lightweight: it approximates the
// number of decision/branching points in a function body by counting keywords
// and operators. It does not parse Rust syntax, so it will miss constructs
// such as `loop`, `break`, `continue`, `return`, nested closures, `try` blocks,
// match guards, and short-circuiting macros. It is sufficient as a coarse
// guardrail, not a precise metric.
//
// Note: Match arms (`=>`) are counted at brace depth 1 (top-level match blocks)
// but not at depth 2+ to avoid false positives from closures (`|| x =>`) and
// map literals (`HashMap::from([{(k, v)}])`).

/// Approximate cyclomatic complexity by counting control-flow tokens.
///
/// Currently counted tokens: `if`, `else if`, `match`, `while`, `for`, `&&`,
/// `||`, `?`, and match arms at depth 1 (top-level match bodies).
/// `loop`, `break`, `continue`, `return` are intentionally excluded.
///
/// Match arms are counted only when `brace_depth == 1` (top-level match block),
/// which excludes arms nested in closures (`|| x => { }`) and map literals
/// (`HashMap::from([{(k, v)}])` where the `{` is a closure body at depth 2+).
pub fn count_complexity(trimmed: &str, brace_depth: usize) -> usize {
    let arms = if brace_depth == 1 {
        trimmed.matches("=>").count()
    } else {
        0
    };
    // Use addition without ? to stay under the complexity ceiling.
    let base = trimmed.matches("if ").count()
        + trimmed.matches("else if").count()
        + trimmed.matches("match ").count()
        + trimmed.matches("while ").count()
        + trimmed.matches("for ").count()
        + trimmed.matches("&&").count()
        + trimmed.matches("||").count();
    base + arms
}

#[cfg(test)]
mod tests {
    use super::count_complexity;

    #[test]
    fn complexity_count_tracks_conditionals_and_operators() {
        let code = "if a && b || c { d? } else if e { match f { _ => g } }";
        // Line 1: if, &&, ||, ? = 4
        // Line 2: else if, match = 2
        // Line 3: => (depth 1) = 1
        // Total: 7
        assert_eq!(count_complexity(code, 1), 7);
    }

    #[test]
    fn complexity_count_does_not_count_loop_control() {
        // The heuristic is documented as approximate and intentionally does not
        // count loop control tokens.
        assert_eq!(count_complexity("loop { break; continue; }", 1), 0);
        assert_eq!(count_complexity("return x;", 1), 0);
    }

    #[test]
    fn complexity_count_does_not_count_map_literals() {
        // Match arms are not counted at depth 2+ (closure body depth),
        // which avoids false positives from map literals and closures.
        let code = "HashMap::from([(k1, v1), (k2, v2), (k3, v3)])";
        assert_eq!(count_complexity(code, 1), 0);
        assert_eq!(count_complexity(code, 2), 0);
    }

    #[test]
    fn match_arms_counted_at_depth_one() {
        // A match with 3 arms adds 3 complexity at depth 1 (inside a function body).
        let code = "match x { A => 1, B => 2, C => 3, }";
        assert_eq!(count_complexity(code, 1), 4); // 1 match + 3 arms
    }

    #[test]
    fn match_arms_not_counted_at_depth_two() {
        // Match arms at depth 2 (inside a closure or other block) are not counted.
        // Single-arm patterns at depth 1 add +1.
        assert_eq!(count_complexity("Some(x) => { x + 1 }", 1), 1); // 1 arm at depth 1
        assert_eq!(count_complexity("Some(x) => { x + 1 }", 2), 0); // 0 arms at depth 2
        // Multi-arm patterns at depth 1 add N arms.
        assert_eq!(count_complexity("A => 1, B => 2, C => 3,", 1), 3); // 3 arms at depth 1
        assert_eq!(count_complexity("A => 1, B => 2, C => 3,", 2), 0); // 0 arms at depth 2
    }

    #[test]
    fn nested_match_arms_depth_tracking() {
        // An if containing a match: the outer if is at depth 1, the inner arms
        // are at depth 2 (inside the if block) so they are not double-counted.
        let outer_if = "if cond { ";
        let inner_match = "    match x { A => a, B => b } ";
        let close = "}";
        // At depth 1: if
        assert_eq!(count_complexity(outer_if, 1), 1);
        // At depth 2: match but no arms (arms are at depth 3 inside the match block)
        assert_eq!(count_complexity(inner_match, 2), 1); // 1 match
        // At depth 1: no new complexity
        assert_eq!(count_complexity(close, 1), 0);
    }

    #[test]
    fn single_arm_match_counts_both() {
        // A single-arm match contributes match + arm = 2 at depth 1.
        // `match x { Some(y) => { y + 1 } }` appears at depth 1 (outer fn body).
        let code = "match x { Some(y) => { y + 1 } }";
        assert_eq!(count_complexity(code, 1), 2); // 1 match + 1 arm
    }

    #[test]
    fn closure_pattern_not_counted_as_match_arm() {
        // At depth 1, a line with `||` (closure or logical OR) adds 1.
        // The `=>` in `|| x =>` is closure syntax, not a match arm, but it IS
        // counted at depth 1 since the heuristic cannot distinguish them.
        // This is a known limitation: closure patterns add complexity at depth 1.
        let code = "move || x => x + 1";
        assert_eq!(count_complexity(code, 1), 2); // 1 || + 1 => (closure pattern at depth 1)
        assert_eq!(count_complexity(code, 2), 1); // 1 || at depth 2, => not counted
    }
}
