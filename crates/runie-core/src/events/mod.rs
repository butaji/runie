//! Event bus + subscriber registry.

pub mod bus;
pub mod subscribe;

pub use bus::EventBus;
pub use subscribe::{SubId, Subscriber, SubscriberRegistry};
