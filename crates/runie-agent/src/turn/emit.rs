//! Emit helpers for agent turn events.

use crate::stream_response::EmitFn;

/// Emit a response message and the Done event.
pub fn emit_response_and_done(emit: &EmitFn, id: &str, content: String) {
    emit(runie_core::Event::response(id, content));
    emit(runie_core::Event::Done { id: id.to_owned() });
}

/// Emit an error message and the Done event.
pub fn emit_error_and_done(emit: &EmitFn, id: &str, message: String) {
    emit(runie_core::Event::Error { id: id.to_owned(), message });
    emit(runie_core::Event::Done { id: id.to_owned() });
}
