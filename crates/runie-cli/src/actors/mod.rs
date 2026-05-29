#![allow(unused_imports)]

// DEPRECATED: Superseded by pipe architecture in Phase 3
#[deprecated(since = "0.1.0", note = "Use pipe module instead")]
pub mod framework;
pub mod input;

// DEPRECATED: These types are unused - superseded by pipe architecture
#[deprecated(since = "0.1.0", note = "Use pipe module instead")]
pub use framework::{
    Actor, ActorEvent, ActorSystem,
    actors::{InputBarActor, MessageListActor, StatusBarActor, TopBarActor},
    events::{InputBarEvent, MessageListEvent, StatusBarEvent, TopBarEvent},
    msgs::{InputBarMsg, MessageListMsg, StatusBarMsg, TopBarMsg},
};
pub use input::InputActor;
