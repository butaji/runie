#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionHistoryQuery {
    pub text: String,
    pub outcome: Option<String>,
    pub offset: usize,
    pub limit: usize,
}

pub fn parse_question_history_query(args: &str) -> QuestionHistoryQuery {
    let mut text = Vec::new();
    let mut query = QuestionHistoryQuery::default();
    for part in args.split_whitespace() {
        if let Some(value) = part.strip_prefix("outcome=") {
            if matches!(value, "answered" | "cancelled" | "rejected") {
                query.outcome = Some(value.to_owned());
                continue;
            }
        }
        if let Some(value) = part.strip_prefix("offset=").and_then(|v| v.parse().ok()) {
            query.offset = value;
            continue;
        }
        if let Some(value) = part.strip_prefix("limit=").and_then(|v| v.parse().ok()) {
            query.limit = value;
            continue;
        }
        text.push(part);
    }
    query.text = text.join(" ");
    query
}

impl Default for QuestionHistoryQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            outcome: None,
            offset: 0,
            limit: 32,
        }
    }
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
                outcome: Some("answered".into()),
                offset: 0,
                limit: 32,
            }
        );
        assert_eq!(
            parse_question_history_query("cancelled deploy"),
            QuestionHistoryQuery {
                text: "cancelled deploy".into(),
                outcome: None,
                offset: 0,
                limit: 32,
            }
        );
    }

    #[test]
    fn parses_bounded_history_page_controls_without_losing_text() {
        assert_eq!(
            parse_question_history_query("outcome=answered offset=4 limit=8 deploy"),
            QuestionHistoryQuery {
                text: "deploy".into(),
                outcome: Some("answered".into()),
                offset: 4,
                limit: 8,
            }
        );
    }
}
