# Split `AppState::tick_animation` (45 lines, 4 concerns) into Focused Methods

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2

## Description

`crates/runie-core/src/model.rs:497-545` has a 45-line
`AppState::tick_animation` method that does 4 unrelated things:

1. **Animation frame counter** (lines 498-510): increments
   `self.animation_frame` if `turn_active`
2. **Speed/throughput calculation** (lines 511-528): rolls the
   `SpeedWindow`, computes tokens/sec
3. **Input flash countdown** (lines 529-534): decrements
   `self.input_flash` while > 0
4. **Transient message expiry** (lines 535-545): clears
   `transient_message` if `transient_until` is in the past

The method then has an `animate_tokens` sub-method (lines
547-580) that does token interpolation (yet another concern).

The 45-line function violates the 40-line strict cap, the 80-line
relaxed cap, and the "one function, one concern" principle.

## Current State

```rust
impl AppState {
    pub fn tick_animation(&mut self) {
        let mut changed = false;
        if self.agent.turn_active {
            self.animation_frame = self.animation_frame.wrapping_add(1);
            self.update_speed();  // calls another 30-line method
            changed = true;
        }
        if self.input.input_flash > 0 {
            self.input.input_flash -= 1;
            changed = true;
        }
        if self.clear_expired_transient() {
            changed = true;
        }
        if self.animate_tokens() {  // 35 lines of token easing
            changed = true;
        }
        if changed {
            self.view.dirty = true;
        }
    }
}
```

Each concern should be its own method, called from a
top-level `tick_animation` orchestrator. This is mechanical
refactoring — no behavior change.

## Acceptance Criteria

- [ ] `tick_animation` is ≤ 20 lines and just orchestrates 4-5
  sub-methods
- [ ] `tick_animation_frame()` (or similar) handles the
  `animation_frame` counter
- [ ] `tick_speed_window()` handles the speed rolling window
- [ ] `tick_input_flash()` decrements `input.input_flash`
- [ ] `tick_transient()` clears expired transients
- [ ] `tick_token_animation()` interpolates token display values
- [ ] `tick_animation` is `pub fn` (called from `runie-term`)
- [ ] No behavior change: same animation rate, same flash
  duration, same speed window
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds (1,631 tests)
- [ ] The existing test `status_timer.rs` (if it tests animation
  timing) still passes

## Tests

### Layer 1 — State/Logic
- [ ] `cargo test -p runie-core --lib tests::status_timer` passes
  (status timer tests)
- [ ] `cargo test -p runie-core --lib tests::token_counters` passes
  (token animation tests)
- [ ] `cargo test -p runie-core --lib update::` passes (dispatcher)

### Layer 4 — Smoke
- [ ] `./target/release/runie` shows spinner animation at 5 fps
  during agent turns (visual verification)

## Notes

**The methods called by `tick_animation` are mostly small:**

```rust
fn clear_expired_transient(&mut self) -> bool {
    // 11 lines, already extracted
    if let Some(until) = self.transient_until {
        if std::time::Instant::now() > until {
            self.transient_message = None;
            self.transient_until = None;
            self.transient_level = None;
            return true;
        }
    }
    false
}

fn update_speed(&mut self) {
    // 32 lines, already extracted (lines 580-618 of model.rs)
    // ... (token throughput calculation)
}

fn animate_tokens(&mut self) -> bool {
    // 35 lines, already extracted
    // ... (easing math for display values)
}
```

The 4 sub-methods already exist (or can be extracted trivially).
The refactor is just splitting the top-level orchestrator.

**Why is this P2:** Mechanical, no design decisions, but
doesn't fix any user-visible bug. Mostly improves readability.

**Out of scope:**
- The 32-line `update_speed` method itself (could be further
  split, separate task)
- The 35-line `animate_tokens` easing math (could use a
  generic easing trait, separate task)
- Performance optimization (this is already 60Hz, no need)
- Adding more `tick_*` methods for other periodic work (the
  `request_queue`, `message_queue`, etc. are not time-driven)

**Verification:**
```bash
# tick_animation is small
awk 'NR==497,NR==600' crates/runie-core/src/model.rs | grep -c "^    pub fn\|^    fn "
# Should be 5-6 methods, not 1

# All sub-methods are short
for m in tick_animation tick_speed_window tick_input_flash tick_transient tick_token_animation; do
  awk "/pub fn $m/,/^    }/" crates/runie-core/src/model.rs | wc -l
done
# All should be < 30

# Build + tests clean
cargo build --workspace
cargo test --workspace
```
