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

#[cfg(test)]
mod tests {
    use super::{parse_git_commit, parse_git_commit_prepare};

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
}
