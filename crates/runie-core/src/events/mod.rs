//! Event bus + subscriber registry.

pub mod bus;
pub mod subscribe;

pub use bus::{EventBus, PiEventReceiver};
pub use subscribe::{SubId, Subscriber, SubscriberRegistry};
