impl ScrollbackActor {
    pub fn shared_snapshot(&self) -> SharedSnapshot<FeedSnapshot> {
        self.shared_snapshot.borrow().clone()
    }

    pub fn shared_subscribe(&self) -> watch::Receiver<SharedSnapshot<FeedSnapshot>> {
        self.shared_snapshot.clone()
    }
}
