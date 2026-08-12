pub fn parse_git_commit_prepare(args: &str) -> Option<&str> {
    let message = args.strip_prefix("commit prepare ")?.trim();
    (!message.is_empty()).then_some(message)
}

pub fn parse_git_commit(args: &str) -> Option<&str> {
    let message = args.strip_prefix("commit ")?.trim();
    if message.starts_with("prepare ") || message.is_empty() {
        None
    } else {
        Some(message)
    }
}

pub fn parse_git_push(args: &str) -> Option<(&str, &str)> {
    let mut parts = args.strip_prefix("push ")?.split_whitespace();
    let remote = parts.next()?;
    let reference = parts.next()?;
    parts.next().is_none().then_some((remote, reference))
}

pub fn parse_git_revert(args: &str) -> Option<&str> {
    let commit = args.strip_prefix("revert ")?.trim();
    (commit.len() >= 7 && !commit.contains(char::is_whitespace)).then_some(commit)
}

#[cfg(test)]
mod tests {
    use super::{parse_git_commit, parse_git_commit_prepare, parse_git_push, parse_git_revert};

    #[test]
    fn commit_prepare_keeps_the_message_as_typed_data() {
        assert_eq!(
            parse_git_commit_prepare("commit prepare ship it"),
            Some("ship it")
        );
        assert!(parse_git_commit_prepare("commit prepare ").is_none());
    }

    #[test]
    fn commit_excludes_prepare_subcommand() {
        assert_eq!(parse_git_commit("commit ship it"), Some("ship it"));
        assert!(parse_git_commit("commit prepare ship it").is_none());
    }

    #[test]
    fn push_and_revert_keep_validated_arguments_typed() {
        assert_eq!(parse_git_push("push origin main"), Some(("origin", "main")));
        assert!(parse_git_push("push origin main extra").is_none());
        assert_eq!(parse_git_revert("revert deadbee"), Some("deadbee"));
        assert!(parse_git_revert("revert nope").is_none());
    }
}
