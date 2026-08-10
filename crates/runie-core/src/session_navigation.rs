impl SessionActor {
    /// Move the actor-owned selected leaf to its parent without deleting any
    /// journal data. The worker computes the target from its current state.
    pub async fn undo(&self) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::Undo { reply: reply_tx })
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }
}
