# Suspend to Background (Ctrl+Z)

**Status**: todo
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

- [ ] `Ctrl+Z` suspends to background on Unix
- [ ] No-op on Windows (no job control)
- [ ] Terminal restored on resume
- [ ] Event loop pauses correctly

## Tests

### Layer 2
- [ ] `ctrl_z_emits_suspend` — keymap event
