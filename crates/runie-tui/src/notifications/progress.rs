//! Terminal tab-progress protocol (Grok OSC 9;4 parity).

use crate::terminal::context::{TerminalContext, TerminalName};
use std::time::{Duration, Instant};

const KEEPALIVE: Duration = Duration::from_secs(5);

pub fn supported(ctx: &TerminalContext) -> bool {
    match ctx.brand {
        TerminalName::Ghostty | TerminalName::WezTerm => true,
        TerminalName::Iterm2 => ctx
            .term_program_version
            .as_deref()
            .is_some_and(|version| version_at_least(version, 3, 6)),
        _ => false,
    }
}

fn version_at_least(version: &str, required_major: u32, required_minor: u32) -> bool {
    let mut parts = version
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    (major, minor) >= (required_major, required_minor)
}

pub fn start_sequence() -> &'static str {
    "\x1b]9;4;1;-1\x07"
}
pub fn clear_sequence() -> &'static str {
    "\x1b]9;4;0;0\x07"
}

#[derive(Debug, Clone, Default)]
pub struct ProgressState {
    active: bool,
    last_emit: Option<Instant>,
}

impl ProgressState {
    /// Return a sequence only when the tab state needs changing or a
    /// keepalive is due. This prevents redraw ticks from spamming the PTY.
    pub fn update(&mut self, ctx: &TerminalContext, busy: bool, now: Instant) -> Option<&'static str> {
        if !supported(ctx) {
            self.active = false;
            self.last_emit = None;
            return None;
        }
        if busy {
            if !self.active
                || self
                    .last_emit
                    .is_none_or(|at| now.duration_since(at) >= KEEPALIVE)
            {
                self.active = true;
                self.last_emit = Some(now);
                return Some(start_sequence());
            }
        } else if self.active {
            self.active = false;
            self.last_emit = None;
            return Some(clear_sequence());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ghostty() -> TerminalContext {
        TerminalContext {
            brand: TerminalName::Ghostty,
            multiplexer: crate::terminal::context::MultiplexerKind::None,
            tmux_version: None,
            term_program_version: None,
        }
    }

    #[test]
    fn emits_start_once_then_keepalive() {
        let mut state = ProgressState::default();
        let t0 = Instant::now();
        assert_eq!(state.update(&ghostty(), true, t0), Some(start_sequence()));
        assert_eq!(
            state.update(&ghostty(), true, t0 + Duration::from_secs(1)),
            None
        );
        assert_eq!(
            state.update(&ghostty(), true, t0 + Duration::from_secs(5)),
            Some(start_sequence())
        );
    }

    #[test]
    fn clears_when_turn_finishes() {
        let mut state = ProgressState::default();
        let t0 = Instant::now();
        state.update(&ghostty(), true, t0);
        assert_eq!(
            state.update(&ghostty(), false, t0 + Duration::from_secs(1)),
            Some(clear_sequence())
        );
        assert_eq!(
            state.update(&ghostty(), false, t0 + Duration::from_secs(2)),
            None
        );
    }

    #[test]
    fn unsupported_terminal_is_noop() {
        let mut state = ProgressState::default();
        let ctx = TerminalContext {
            brand: TerminalName::Unknown,
            multiplexer: crate::terminal::context::MultiplexerKind::None,
            tmux_version: None,
            term_program_version: None,
        };
        assert_eq!(state.update(&ctx, true, Instant::now()), None);
    }

    #[test]
    fn iterm_requires_version_36() {
        let old = TerminalContext {
            brand: TerminalName::Iterm2,
            multiplexer: crate::terminal::context::MultiplexerKind::None,
            tmux_version: None,
            term_program_version: Some("3.5.9".into()),
        };
        let new = TerminalContext { term_program_version: Some("3.6.0".into()), ..old.clone() };
        let missing = TerminalContext { term_program_version: None, ..old };
        assert!(!supported(&missing));
        assert!(!supported(&TerminalContext {
            term_program_version: Some("3.5.9".into()),
            ..missing.clone()
        }));
        assert!(supported(&new));
    }
}
