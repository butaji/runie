impl UserQuestionBroker {
    /// Ask an ordered batch through the same owned queue; each next question
    /// is published only after the previous answer settles.
    pub async fn ask_many(
        &self,
        requests: impl IntoIterator<Item = UserQuestionRequest>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut answers = Vec::new();
        for request in requests {
            answers.push(self.ask(request).await?);
        }
        Ok(answers)
    }
}
