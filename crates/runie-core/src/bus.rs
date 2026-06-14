use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct EventBus<T> {
    tx: broadcast::Sender<T>,
}

impl<T> EventBus<T>
where
    T: Clone + Send + 'static,
{
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: T) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.tx.subscribe()
    }
}
