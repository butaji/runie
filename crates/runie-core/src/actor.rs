use crate::bus::EventBus;
use tokio::sync::mpsc;

pub trait Actor: Sized {
    type Msg: Send + 'static;
    type Event: Clone + Send + 'static;

    async fn run(
        self,
        rx: mpsc::Receiver<Self::Msg>,
        bus: EventBus<Self::Event>,
    );
}
