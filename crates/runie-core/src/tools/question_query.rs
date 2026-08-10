#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionHistoryQuery {
    pub text: String,
    pub outcome: Option<String>,
}

pub fn parse_question_history_query(args: &str) -> QuestionHistoryQuery {
    let mut parts = args.split_whitespace();
    let first = parts.next().unwrap_or_default();
    let outcome = first
        .strip_prefix("outcome=")
        .filter(|value| matches!(*value, "answered" | "cancelled" | "rejected"))
        .map(str::to_owned);
    let text = if outcome.is_some() {
        parts.collect::<Vec<_>>().join(" ")
    } else {
        args.trim().to_owned()
    };
    QuestionHistoryQuery { text, outcome }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_optional_outcome_prefix() {
        assert_eq!(
            parse_question_history_query("outcome=answered deploy"),
            QuestionHistoryQuery {
                text: "deploy".into(),
                outcome: Some("answered".into())
            }
        );
        assert_eq!(
            parse_question_history_query("cancelled deploy"),
            QuestionHistoryQuery {
                text: "cancelled deploy".into(),
                outcome: None
            }
        );
    }
}
