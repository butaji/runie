//! Typed messages for `ViewActor`.

use crate::actors::GenericActorHandle;

use crate::model::InputReceiver;

/// All messages accepted by `ViewActor`.
///
/// Covers scroll, cache invalidation, terminal sizing, dialog state, and vim nav.
#[derive(Debug, Clone)]
pub enum ViewMsg {
    // ── Cache invalidation ─────────────────────────────────────────────────
    /// Invalidate the view cache (dirty flag).
    Invalidate,
    /// Messages changed — rebuild feed cache.
    MessagesChanged,

    // ── Scroll ─────────────────────────────────────────────────────────────
    /// Scroll by N lines in given direction.
    Scroll { direction: ScrollDirection },
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Go to top.
    GoToTop,
    /// Go to bottom.
    GoToBottom,
    /// Jump to next/previous element.
    ElementJump { direction: ElementJumpDirection },

    // ── Mouse ──────────────────────────────────────────────────────────────
    /// Mouse moved to position.
    MouseMoved { row: u16, col: u16 },
    /// Mouse clicked at position.
    MouseClicked { row: u16, col: u16, button: MouseButton },
    /// Mouse released.
    MouseReleased { row: u16, col: u16, button: MouseButton },

    // ── Terminal sizing ───────────────────────────────────────────────────
    /// Terminal was resized.
    TerminalSized { width: u16, height: u16 },

    // ── Dialog state ──────────────────────────────────────────────────────
    /// A dialog was opened — update input receiver.
    DialogOpened,
    /// The active dialog was closed — restore input receiver.
    DialogClosed,

    // ── Input receiver ────────────────────────────────────────────────────
    /// Set the keyboard input receiver.
    SetInputReceiver { receiver: InputReceiver },

    // ── Vim navigation ───────────────────────────────────────────────────
    /// Enable/disable vim nav mode with optional selected post.
    VimNav { enabled: bool, selected_post: Option<usize> },
    /// Toggle expand/collapse all items.
    ToggleExpandAll,

    // ── Turn lifecycle ───────────────────────────────────────────────────
    /// Turn ended successfully — clear turn state.
    TurnEnded,
    /// Turn errored — set vim nav pending.
    TurnErrored,

    // ── Animation ─────────────────────────────────────────────────────────
    /// Animation tick — increment frame counter.
    AnimationTick,
}

/// Scroll direction.
#[derive(Debug, Clone, Copy)]
pub enum ScrollDirection {
    Up,
    Down,
    HalfUp,
    HalfDown,
}

/// Element jump direction.
#[derive(Debug, Clone, Copy)]
pub enum ElementJumpDirection {
    Next,
    Prev,
}

/// Mouse button.
#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Handle for sending messages to `ViewActor`.
pub type ViewActorHandle = GenericActorHandle<ViewMsg>;
