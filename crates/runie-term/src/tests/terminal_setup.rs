//! Tests for terminal setup helpers in the binary crate root.

#[test]
fn push_keyboard_enhancement_flags_writes_sequence() {
    let mut buf = Vec::new();
    crate::terminal_setup::push_keyboard_enhancement_flags(&mut buf).unwrap();
    assert!(!buf.is_empty(), "helper must emit a non-empty sequence");
    let s = String::from_utf8_lossy(&buf);
    // Crossterm encodes DISAMBIGUATE_ESCAPE_CODES | REPORT_ALL_KEYS_AS_ESCAPE_CODES
    // as the kitty keyboard protocol push sequence: CSI > 9 u.
    assert!(
        s.contains("\u{1b}[>9u"),
        "must push keyboard enhancement flags, got {:?}",
        s
    );
}

#[test]
fn push_keyboard_enhancement_flags_error_is_err() {
    struct FailingWriter;
    impl std::io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "unsupported",
            ))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut writer = FailingWriter;
    assert!(
        crate::terminal_setup::push_keyboard_enhancement_flags(&mut writer).is_err(),
        "helper must surface writer errors so setup_terminal can ignore them"
    );
}
