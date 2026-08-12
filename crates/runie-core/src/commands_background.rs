pub fn parse_background_job_output_preview_query(args: &str) -> Option<&str> {
    let mut parts = args.split_whitespace();
    (parts.next() == Some("output")
        && parts.next().is_some()
        && parts.next() == Some("preview")
        && parts.next().is_none())
    .then(|| args.split_whitespace().nth(1).unwrap())
}

#[cfg(test)]
mod commands_background_tests {
    use super::parse_background_job_output_preview_query;

    #[test]
    fn preview_query_is_exact_and_keeps_the_job_id_as_data() {
        assert_eq!(
            parse_background_job_output_preview_query("output job-7 preview"),
            Some("job-7")
        );
        for invalid in ["output preview", "output job-7", "output job-7 preview extra"] {
            assert_eq!(parse_background_job_output_preview_query(invalid), None);
        }
    }
}
