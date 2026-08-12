pub fn parse_git_commit_prepare(args: &str) -> Option<&str> {
    let message = args.strip_prefix("commit prepare ")?.trim();
    (!message.is_empty()).then_some(message)
}

#[cfg(test)]
mod tests {
    use super::parse_git_commit_prepare;

    #[test]
    fn commit_prepare_keeps_the_message_as_typed_data() {
        assert_eq!(
            parse_git_commit_prepare("commit prepare ship it"),
            Some("ship it")
        );
        assert!(parse_git_commit_prepare("commit prepare ").is_none());
    }
}
