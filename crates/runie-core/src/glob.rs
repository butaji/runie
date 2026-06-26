//! Simple glob pattern matching.
//!
//! Provides basic glob pattern matching for tool names and paths.
//! Supports `*` (match any chars except `/`) and `**` (match any chars including `/`).

/// Match a string against a glob pattern.
///
/// Supported patterns:
/// - `*` matches any sequence of characters except `/`
/// - `**` matches any sequence of characters including `/` (can match empty)
/// - `?` matches any single character except `/`
/// - Other characters match themselves
pub fn matches(pattern: &str, name: &str) -> bool {
    do_match(pattern.as_bytes(), name.as_bytes())
}

fn do_match(pat: &[u8], name: &[u8]) -> bool {
    match (pat.first(), name.first()) {
        (None, None) => true,
        (None, Some(&b'/')) => true,
        (None, Some(_)) => false,
        (Some(_), None) => all_stars(pat),
        _ => continue_match(pat, name),
    }
}

fn all_stars(pat: &[u8]) -> bool {
    pat.iter().all(|&c| c == b'*')
}

fn continue_match(pat: &[u8], name: &[u8]) -> bool {
    match (pat.first(), name.first()) {
        (Some(&b'*'), _) if pat.starts_with(b"**") => double_star(pat, name),
        (Some(&b'*'), _) => single_star(pat, name),
        (Some(&b'?'), _) => question(name) && do_match(&pat[1..], &name[1..]),
        (Some(pc), Some(nc)) => pc == nc && do_match(&pat[1..], &name[1..]),
        _ => false,
    }
}

fn double_star(pat: &[u8], name: &[u8]) -> bool {
    let rest = &pat[2..];
    // Try ** matching 0 chars, then rest of pattern
    if do_match(rest, name) {
        return true;
    }
    // Try ** matching 1+ chars (including /)
    for skip in 1..=name.len() {
        if do_match(rest, &name[skip..]) {
            return true;
        }
    }
    false
}

fn single_star(pat: &[u8], name: &[u8]) -> bool {
    do_match(&pat[1..], name) || {
        let mut skip = 1;
        while skip <= name.len() && name[skip - 1] != b'/' {
            if do_match(&pat[1..], &name[skip..]) {
                return true;
            }
            skip += 1;
        }
        false
    }
}

fn question(name: &[u8]) -> bool {
    name.first().is_some_and(|c| c != &b'/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        assert!(matches("hello", "hello"));
        assert!(!matches("hello", "world"));
    }

    #[test]
    fn star_matches_any() {
        assert!(matches("*", "anything"));
        assert!(matches("read_*", "read_file"));
        assert!(!matches("read_*", "read"));
        assert!(!matches("read_*", "write_file"));
    }

    #[test]
    fn star_does_not_match_slash() {
        assert!(!matches("*.rs", "src/main.rs"));
        assert!(matches("*.rs", "main.rs"));
    }

    #[test]
    fn double_star_matches_slash() {
        // **/*.rs matches paths that have / before the .rs file
        assert!(matches("**/*.rs", "src/main.rs"));
        assert!(matches("**/*.rs", "/main.rs"));
        // main.rs doesn't have a /, so **/*.rs can't match it
        assert!(!matches("**/*.rs", "main.rs"));
    }

    #[test]
    fn question_mark() {
        assert!(matches("file?.rs", "file1.rs"));
        assert!(!matches("file?.rs", "file12.rs"));
    }

    #[test]
    fn sensitive_paths() {
        // **/.env requires a / before .env (like ./env or path/to/.env)
        assert!(!matches("**/.env", ".env")); // .env has no leading /
        assert!(matches(".env", ".env"));
        assert!(matches("**/.ssh/*", "src/.ssh/config"));
        assert!(matches(".ssh/*", ".ssh/config"));
        assert!(!matches("**/.ssh/*", ".ssh")); // needs something after .ssh/
    }
}
