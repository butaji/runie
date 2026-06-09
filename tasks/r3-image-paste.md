# Image Paste (Ctrl+V)

**Status**: todo
**Milestone**: R3
**Category**: Input & Commands

## Description

Paste images from clipboard. Converts to base64 and sends as vision model input.

## Architecture

```rust
fn cmd_paste_image(_args: &str) -> Option<Event> {
    Some(Event::PasteImage)
}

async fn read_clipboard_image() -> Result<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("osascript")
            .args(["-e", "the clipboard as «class PNGf»"])
            .output()?;
        // Parse AppleScript output
    }
    #[cfg(target_os = "linux")]
    {
        // Try wl-copy or xclip
    }
    #[cfg(target_os = "windows")]
    {
        // Use clipboard-win or arboard
    }
}
```

### Events

```rust
Event::PasteImage,  // Ctrl+V (Alt+V on Win)
```

## Acceptance Criteria

- [ ] `Ctrl+V` pastes image from clipboard
- [ ] `Alt+V` on Windows
- [ ] Converts image to base64
- [ ] Shows image attachment in input
- [ ] Sends to vision-capable models
- [ ] Falls back gracefully if clipboard has text
- [ ] Max 5MB per image

## Tests

### Layer 2
- [ ] `ctrl_v_emits_paste_image` — keymap event
