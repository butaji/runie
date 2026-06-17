// Build lint helper shared between `build.rs` and the test suite.
//
// The complexity heuristic is intentionally lightweight: it approximates the
// number of decision/branching points in a function body by counting keywords
// and operators. It does not parse Rust syntax, so it will miss constructs
// such as `loop`, `break`, `continue`, `return`, nested closures, `try` blocks,
// match guards, and short-circuiting macros. It is sufficient as a coarse
// guardrail, not a precise metric.

/// Approximate cyclomatic complexity by counting control-flow tokens.
///
/// Currently counted tokens: `if`, `else if`, `match`, `while`, `for`, `&&`,
/// `||`, and `?`. `loop`, `break`, `continue`, and `return` are intentionally
/// excluded because they occur in most async/production functions and would
/// produce excessive false positives under the current thresholds.
pub fn count_complexity(trimmed: &str) -> usize {
    trimmed.matches("if ").count()
        + trimmed.matches("else if").count()
        + trimmed.matches("match ").count()
        + trimmed.matches("while ").count()
        + trimmed.matches("for ").count()
        + trimmed.matches("&&").count()
        + trimmed.matches("||").count()
        + trimmed.matches('?').count()
}

#[cfg(test)]
mod tests {
    use super::count_complexity;

    #[test]
    fn complexity_count_tracks_conditionals_and_operators() {
        let code = "if a && b || c { d? } else if e { match f { _ => g } }";
        // if, &&, ||, ?, else if (and its embedded "if "), match = 7
        assert_eq!(count_complexity(code), 7);
    }

    #[test]
    fn complexity_count_does_not_count_loop_control() {
        // The heuristic is documented as approximate and intentionally does not
        // count loop control tokens.
        assert_eq!(count_complexity("loop { break; continue; }"), 0);
        assert_eq!(count_complexity("return x;"), 0);
    }
}
