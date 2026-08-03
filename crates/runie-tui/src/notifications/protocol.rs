//! Pure notification protocol selection and sequence construction.

use crate::terminal::context::{MultiplexerKind, TerminalContext, TerminalName};
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationProtocol {
    Osc9,
    Osc99,
    Osc777,
    Bel,
    None,
}

pub fn select_protocol(ctx: &TerminalContext) -> NotificationProtocol {
    if ctx.multiplexer == MultiplexerKind::Zellij {
        return NotificationProtocol::Bel;
    }
    match ctx.brand {
        TerminalName::Iterm2 | TerminalName::WezTerm | TerminalName::Warp => NotificationProtocol::Osc9,
        TerminalName::Kitty => NotificationProtocol::Osc99,
        TerminalName::Ghostty | TerminalName::Vte | TerminalName::Foot => NotificationProtocol::Osc777,
        TerminalName::Unknown | TerminalName::AppleTerminal | TerminalName::Alacritty => NotificationProtocol::Bel,
    }
}

pub fn notification_sequence(protocol: NotificationProtocol, title: &str, body: &str) -> Option<String> {
    let title = sanitize(title);
    let body = sanitize(body);
    Some(match protocol {
        NotificationProtocol::Osc9 => format!("\x1b]9;{body} · {title}\x07"),
        NotificationProtocol::Osc99 => format!("\x1b]99;i=grok;{body} · {title}\x1b\\"),
        NotificationProtocol::Osc777 => format!("\x1b]777;notify;Grok;{body}\x1b\\"),
        NotificationProtocol::Bel => "\x07".to_owned(),
        NotificationProtocol::None => return None,
    })
}

fn sanitize(value: &str) -> String {
    value.replace(['\x07', '\x1b', '\n', '\r'], " ")
}

/// Write one notification using the best protocol for the detected terminal.
/// tmux needs DCS passthrough so the outer terminal can receive the OSC.
pub fn emit_notification<W: Write>(writer: &mut W, ctx: &TerminalContext, title: &str, body: &str) -> io::Result<()> {
    let protocol = select_protocol(ctx);
    let Some(sequence) = notification_sequence(protocol, title, body) else {
        return Ok(());
    };
    let sequence = if crate::notifications::tmux::passthrough_available(ctx) {
        crate::notifications::tmux::tmux_passthrough(&sequence)
    } else {
        sequence
    };
    writer.write_all(sequence.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_grok_protocol_sequences() {
        assert_eq!(
            notification_sequence(NotificationProtocol::Osc9, "runie", "Done"),
            Some("\x1b]9;Done · runie\x07".into())
        );
        assert_eq!(
            notification_sequence(NotificationProtocol::Osc99, "runie", "Done"),
            Some("\x1b]99;i=grok;Done · runie\x1b\\".into())
        );
        assert_eq!(
            notification_sequence(NotificationProtocol::Osc777, "runie", "Done"),
            Some("\x1b]777;notify;Grok;Done\x1b\\".into())
        );
        assert_eq!(
            notification_sequence(NotificationProtocol::None, "runie", "Done"),
            None
        );
    }

    #[test]
    fn emits_sanitized_tmux_notification() {
        let ctx = TerminalContext {
            brand: TerminalName::Iterm2,
            multiplexer: MultiplexerKind::Tmux,
            tmux_version: Some("3.3.0".into()),
            term_program_version: None,
        };
        let mut out = Vec::new();
        emit_notification(&mut out, &ctx, "Runie\n", "Done\x1b").unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "\x1bPtmux;\x1b\x1b]9;Done  · Runie \x07\x1b\\"
        );
    }
}
