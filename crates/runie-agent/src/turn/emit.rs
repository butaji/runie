//! Emit helpers for agent turn events.

use crate::stream_response::EmitFn;
use runie_core::Event;

/// Emit a response message and the Done event.
pub fn emit_response_and_done(emit: &EmitFn, id: &str, content: String) {
    emit_now(
        emit,
        runie_core::Event::Response {
            id: id.to_string(),
            content,
        },
    );
    emit_now(emit, runie_core::Event::Done { id: id.to_string() });
}

/// Emit an error message and the Done event.
pub fn emit_error_and_done(emit: &EmitFn, id: &str, message: String) {
    emit_now(
        emit,
        runie_core::Event::Error {
            id: id.to_string(),
            message,
        },
    );
    emit_now(emit, runie_core::Event::Done { id: id.to_string() });
}

/// Emit an event through the emit function.
pub fn emit_now(emit: &EmitFn, event: Event) {
    let mut emit = emit.lock().unwrap_or_else(|p| p.into_inner());
    emit(event);
}
