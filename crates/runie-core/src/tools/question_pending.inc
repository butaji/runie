impl PendingUserQuestion {
    pub fn terminal_line(&self) -> String {
        format!("{} · pending · {}", self.id, self.request.question)
    }
}

impl UserQuestionBroker {
    pub fn pending_by_id(&self, id: &str) -> Option<PendingUserQuestion> {
        self.requests
            .lock()
            .expect("question requests lock")
            .get(id)
            .cloned()
            .map(|request| PendingUserQuestion {
                id: id.to_owned(),
                request,
            })
    }

    pub fn pending(&self) -> Vec<PendingUserQuestion> {
        let mut pending = self
            .requests
            .lock()
            .expect("question requests lock")
            .iter()
            .map(|(id, request)| PendingUserQuestion {
                id: id.clone(),
                request: request.clone(),
            })
            .collect::<Vec<_>>();
        pending.sort_by(|left, right| left.id.cmp(&right.id));
        pending
    }
}
