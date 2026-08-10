impl SessionActor {
    pub fn shared_snapshot(&self) -> crate::SharedSnapshot<SessionSnapshot> {
        self.shared_snapshot.borrow().clone()
    }

    pub fn shared_subscribe(
        &self,
    ) -> tokio::sync::watch::Receiver<crate::SharedSnapshot<SessionSnapshot>> {
        self.shared_snapshot.clone()
    }
}
