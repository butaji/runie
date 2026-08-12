pub fn parse_git_commit_prepare(args: &str) -> Option<&str> {
    let message = args.strip_prefix("commit prepare ")?.trim();
    (!message.is_empty()).then_some(message)
}
