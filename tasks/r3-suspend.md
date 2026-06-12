# Suspend to Background (Ctrl+Z)

**Status**: done
**Milestone**: R3
**Category**: Input & Commands

## Description

Suspend runie to shell background with Ctrl+Z, resume with `fg`.

## Architecture

```rust
fn cmd_suspend(_args: &str) -> Option<Event> {
    Some(Event::Suspend)
}

// In main.rs event loop
Event::Suspend => {
    // Raise SIGTSTP
    #[cfg(unix)]
    {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;
        signal::kill(Pid::this(), Signal::SIGTSTP).ok();
    }
}
```

## Acceptance Criteria

- [x] `Ctrl+Z` suspends to background on Unix
- [x] No-op on Windows (no job control)
- [x] Terminal restored on resume
- [x] Event loop pauses correctly

## Tests

### Layer 2
- [x] `ctrl_z_emits_suspend` — keymap event
