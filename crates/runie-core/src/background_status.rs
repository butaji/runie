use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

macro_rules! background_status_wire_names {
    ($(($variant:ident, $wire:literal)),+ $(,)?) => {
        impl BackgroundStatus {
            pub const fn wire_name(&self) -> &'static str { match self { $(Self::$variant => $wire,)+ } }
            pub fn from_wire_name(name: &str) -> Option<Self> { match name { $($wire => Some(Self::$variant),)+ _ => None } }
        }
    };
}

background_status_wire_names! {
    (Running, "running"),
    (Completed, "completed"),
    (Failed, "failed"),
    (Cancelled, "cancelled"),
}
