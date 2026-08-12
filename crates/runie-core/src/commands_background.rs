pub fn parse_background_job_output_preview_query(args: &str) -> Option<&str> {
    let mut parts = args.split_whitespace();
    (parts.next() == Some("output")
        && parts.next().is_some()
        && parts.next() == Some("preview")
        && parts.next().is_none())
    .then(|| args.split_whitespace().nth(1).unwrap())
}
