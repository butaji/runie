use super::{UserQuestionHistoryPage, UserQuestionHistoryRow};

impl UserQuestionHistoryPage {
    pub fn terminal_lines(&self) -> Vec<String> {
        let mut lines = self
            .rows
            .iter()
            .map(UserQuestionHistoryRow::terminal_line)
            .collect::<Vec<_>>();
        if !self.rows.is_empty() {
            lines.push(format!(
                "history offset={} limit={} more={}",
                self.offset, self.limit, self.has_more
            ));
        }
        lines
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

    #[test]
    fn query_is_replayable_data() {
        let query = parse_question_history_query("outcome=cancelled offset=2 limit=4 deploy");
        let restored: QuestionHistoryQuery =
            serde_json::from_value(serde_json::to_value(&query).unwrap()).unwrap();
        assert_eq!(restored, query);
    }

    #[test]
    fn history_page_owns_rows_and_pagination_terminal_projection() {
        let page = UserQuestionHistoryPage {
            offset: 4,
            limit: 2,
            has_more: true,
            rows: vec![UserQuestionHistoryRow {
                id: "q1".into(),
                question: "Continue?".into(),
                outcome: "answered".into(),
                detail: None,
            }],
        };
        assert_eq!(page.terminal_lines().len(), 2);
        assert!(page.terminal_lines()[1].contains("offset=4"));
    }
}
