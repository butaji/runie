/// A single entry in the Orchestrator's dialogue log with the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueEntry {
    /// The Orchestrator asked a question via `ask_user`.
    Question(String),
    /// The user's answer to the preceding question.
    Answer(String),
}

/// Working memory for the Orchestrator during Team mode planning.
///
/// Accumulates questions sent via `ask_user` and their answers so the plan
/// can be refined in one shot before submission.
#[derive(Debug, Clone, Default)]
pub struct OrchestratorContext {
    /// Chronological log of questions and answers.
    dialogue: Vec<DialogueEntry>,
}

impl OrchestratorContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self {
            dialogue: Vec::new(),
        }
    }

    /// Record a question that was sent to the user.
    pub fn record_question(&mut self, question: impl Into<String>) {
        self.dialogue.push(DialogueEntry::Question(question.into()));
    }

    /// Record the user's answer to the most recent pending question.
    pub fn record_answer(&mut self, answer: impl Into<String>) {
        self.dialogue.push(DialogueEntry::Answer(answer.into()));
    }

    /// All dialogue entries.
    pub fn dialogue(&self) -> &[DialogueEntry] {
        &self.dialogue
    }

    /// Questions that have been asked but not yet answered.
    pub fn pending_questions(&self) -> Vec<&str> {
        // Scan forward: every Question that has no subsequent Answer yet is pending.
        let mut pending = Vec::new();
        for entry in &self.dialogue {
            match entry {
                DialogueEntry::Question(q) => pending.push(q.as_str()),
                DialogueEntry::Answer(_) => {
                    if !pending.is_empty() {
                        pending.remove(pending.len() - 1);
                    }
                }
            }
        }
        pending
    }

    /// Whether there are any unanswered questions.
    pub fn has_pending_questions(&self) -> bool {
        !self.pending_questions().is_empty()
    }

    /// Whether the dialogue log is empty.
    pub fn is_empty(&self) -> bool {
        self.dialogue.is_empty()
    }

    /// Number of entries in the dialogue log.
    pub fn len(&self) -> usize {
        self.dialogue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orchestrator_context_records_dialogue() {
        let mut ctx = OrchestratorContext::new();
        assert!(ctx.dialogue().is_empty());
        ctx.record_question("Which file?");
        assert_eq!(ctx.dialogue().len(), 1);
        assert!(matches!(ctx.dialogue()[0], DialogueEntry::Question(_)));
        ctx.record_answer("src/lib.rs");
        assert_eq!(ctx.dialogue().len(), 2);
        assert!(matches!(ctx.dialogue()[1], DialogueEntry::Answer(_)));
    }

    #[test]
    fn orchestrator_context_pending_questions() {
        let mut ctx = OrchestratorContext::new();
        assert!(ctx.pending_questions().is_empty());
        ctx.record_question("Scope?");
        ctx.record_question("Priority?");
        assert_eq!(ctx.pending_questions().len(), 2);
        ctx.record_answer("Large");
        assert_eq!(ctx.pending_questions().len(), 1); // second still pending
        ctx.record_answer("High");
        assert!(ctx.pending_questions().is_empty());
    }

    #[test]
    fn orchestrator_context_has_pending() {
        let mut ctx = OrchestratorContext::new();
        assert!(!ctx.has_pending_questions());
        ctx.record_question("Q1");
        assert!(ctx.has_pending_questions());
    }
}
