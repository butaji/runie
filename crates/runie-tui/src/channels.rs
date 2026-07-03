//! Centralized TUI channel capacities and buffer sizes.
//!
//! All channel capacities are defined here to ensure consistent sizing
//! and make adjustments trivial.

/// Capacity of the effect forwarder channel between UiActor and the render loop.
/// This should be large enough to buffer bursts of events during rendering.
pub const EFFECT_FORWARDER_CHANNEL_CAPACITY: usize = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_forwarder_capacity_is_reasonable() {
        const _: () = assert!(
            EFFECT_FORWARDER_CHANNEL_CAPACITY > 0,
            "channel capacity must be positive"
        );
        const _: () = assert!(
            EFFECT_FORWARDER_CHANNEL_CAPACITY <= 1024,
            "channel capacity must be reasonable"
        );
    }
}
