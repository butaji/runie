//! Ephemeral tip state machine — port of Grok's `tips/ephemeral.rs`.
//!
//! A single-slot, TTL'd, seen-count-gated hint line rendered in the hints
//! row. Tips are dedup-keyed (re-show refreshes the TTL), gated by a
//! per-session in-memory seen-count map, and cleared by TTL expiry, prompt
//! submission (`clear_on_submit`), or an explicit keyed clear.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A single styled span within a tip line (dim body, bold chord/command).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipSpan {
    pub text: String,
    pub bold: bool,
}

impl TipSpan {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), bold: false }
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

/// Default tip TTL ≈ 3 s (Grok: 90 ticks at 30 fps).
pub const DEFAULT_TIP_TTL: Duration = Duration::from_millis(3000);
/// Long tip TTL ≈ 20 s (Grok: 600 ticks at 30 fps).
pub const LONG_TIP_TTL: Duration = Duration::from_millis(20_000);

/// A live tip occupying the single slot.
#[derive(Debug, Clone)]
pub struct EphemeralTip {
    pub key: String,
    pub spans: Vec<TipSpan>,
    pub expires_at: Instant,
    pub seen_cap: u32,
    pub ambient: bool,
}

impl EphemeralTip {
    pub fn new(key: &str, spans: Vec<TipSpan>) -> Self {
        Self {
            key: key.to_owned(),
            spans,
            expires_at: Instant::now() + DEFAULT_TIP_TTL,
            seen_cap: u32::MAX,
            ambient: false,
        }
    }

    pub fn with_seen_cap(mut self, cap: u32) -> Self {
        self.seen_cap = cap;
        self
    }

    pub fn ambient(mut self) -> Self {
        self.ambient = true;
        self
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.expires_at = Instant::now() + ttl;
        self
    }
}

/// Per-session seen record for a tip key: `(count, cap)` — the cap recorded
/// at the first show wins, so a later call cannot bypass the gate by passing
/// a different cap.
pub type TipSeen = (u32, u32);

/// Single-slot tip holder with the Grok state-machine semantics.
#[derive(Debug, Clone, Default)]
pub struct EphemeralTipState {
    pub slot: Option<EphemeralTip>,
}

impl EphemeralTipState {
    pub fn is_active(&self) -> bool {
        self.slot.is_some()
    }

    pub fn current_key(&self) -> Option<&str> {
        self.slot.as_ref().map(|t| t.key.as_str())
    }

    /// Show a tip (Grok `show()` semantics):
    /// - Same key already on screen → refresh TTL, return false (no seen
    ///   gate, no count increment — a visible tip never goes dark mid-TTL).
    /// - Seen-gated tip whose count ≥ cap → no-op, count NOT incremented.
    /// - Different key → replaces the current tip.
    pub fn show(&mut self, tip: EphemeralTip, seen_counts: &mut HashMap<String, TipSeen>) -> bool {
        if let Some(current) = &self.slot {
            if current.key == tip.key {
                self.slot = Some(tip);
                return false;
            }
        }
        let entry = seen_counts.entry(tip.key.clone()).or_insert((0, tip.seen_cap));
        if entry.0 >= entry.1 {
            return false;
        }
        entry.0 += 1;
        self.slot = Some(tip);
        true
    }

    /// Advance TTL; returns true once when the tip expires.
    pub fn tick(&mut self) -> bool {
        if let Some(tip) = &self.slot {
            if Instant::now() >= tip.expires_at {
                self.slot = None;
                return true;
            }
        }
        false
    }

    /// Clear only a matching key.
    pub fn clear(&mut self, key: &str) -> bool {
        if let Some(tip) = &self.slot {
            if tip.key == key {
                self.slot = None;
                return true;
            }
        }
        false
    }

    /// Clear any tip.
    pub fn clear_all(&mut self) -> bool {
        let had = self.slot.is_some();
        self.slot = None;
        had
    }

    /// Clear the tip on prompt submission unless it is `ambient`.
    pub fn clear_on_submit(&mut self) -> bool {
        if let Some(tip) = &self.slot {
            if !tip.ambient {
                self.slot = None;
                return true;
            }
        }
        false
    }
}

/// Minimum terminal height for the tip row to render (Grok
/// `SHORT_TERMINAL_ROWS = 16`).
pub const SHORT_TERMINAL_ROWS: u16 = 16;

// ── Tip kinds (copy verbatim from Grok; chord derived from real binding) ──

/// Plan-nudge keywords (whole-word, case-insensitive; `explain`, `planet`,
/// `redesign` must NOT match).
pub const PLAN_NUDGE_KEYWORDS: &[&str] = &[
    "plan",
    "planning",
    "design",
    "architect",
    "step by step",
    "break this down",
    "lay out",
    "approach",
    "strategy",
];

/// Whole-word, case-insensitive match of the plan keywords against the input.
pub fn plan_nudge_matches(input: &str) -> bool {
    let lower = input.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    PLAN_NUDGE_KEYWORDS.iter().any(|kw| {
        if kw.contains(' ') {
            // Phrase: consecutive whole tokens equal the keyword.
            let parts: Vec<&str> = kw.split_whitespace().collect();
            tokens.windows(parts.len()).any(|w| w == parts)
        } else {
            tokens.contains(kw)
        }
    })
}

/// Builder: plan-nudge tip (`Planning? Check out plan mode via shift+tab`).
pub fn plan_nudge_tip() -> EphemeralTip {
    EphemeralTip::new(
        "plan_nudge",
        vec![
            TipSpan::new("Planning? Check out "),
            TipSpan::new("plan mode").bold(),
            TipSpan::new(" via "),
            TipSpan::new("shift+tab"),
        ],
    )
    .with_seen_cap(3)
}

/// Builder: send-now tip (`Queued · Enter to send now`).
pub fn send_now_tip() -> EphemeralTip {
    EphemeralTip::new(
        "send_now_tip",
        vec![
            TipSpan::new("Queued · "),
            TipSpan::new("Enter").bold(),
            TipSpan::new(" to send now"),
        ],
    )
    .with_seen_cap(3)
}

/// Builder: small-screen tip (`Tight on space? Try /compact-mode`) — ambient
/// (survives submit), seen cap 1/session.
pub fn small_screen_tip() -> EphemeralTip {
    EphemeralTip::new(
        "small_screen_tip",
        vec![
            TipSpan::new("Tight on space? Try "),
            TipSpan::new("/compact-mode").bold(),
        ],
    )
    .with_seen_cap(1)
    .ambient()
}

/// Renderability gate shared by the show triggers and the snapshot projection
/// (Grok `tip_row_renderable`): no occluding view and a tall-enough terminal.
pub fn tip_row_renderable(dialog_open: bool, permission_pending: bool, terminal_rows: u16) -> bool {
    !dialog_open && !permission_pending && terminal_rows > SHORT_TERMINAL_ROWS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(text: &str) -> Vec<TipSpan> {
        vec![TipSpan::new(text)]
    }

    #[test]
    fn show_returns_true_first_time() {
        let mut state = EphemeralTipState::default();
        let mut counts: HashMap<String, TipSeen> = HashMap::new();
        assert!(state.show(EphemeralTip::new("k", spans("hi")), &mut counts));
        assert_eq!(state.current_key(), Some("k"));
        assert_eq!(counts.get("k"), Some(&(1, u32::MAX)));
    }

    #[test]
    fn same_key_refreshes_without_gate_or_count() {
        let mut state = EphemeralTipState::default();
        let mut counts: HashMap<String, TipSeen> = HashMap::new();
        state.show(EphemeralTip::new("k", spans("a")).with_seen_cap(1), &mut counts);
        assert_eq!(counts.get("k"), Some(&(1, 1)));
        // A same-key refresh while visible always succeeds.
        assert!(!state.show(EphemeralTip::new("k", spans("c")), &mut counts));
        assert_eq!(counts.get("k"), Some(&(1, 1)));
    }

    #[test]
    fn seen_cap_blocks_after_replacement() {
        let mut state = EphemeralTipState::default();
        let mut counts: HashMap<String, TipSeen> = HashMap::new();
        assert!(state.show(EphemeralTip::new("k", spans("a")).with_seen_cap(1), &mut counts));
        // Force replacement: different key.
        assert!(state.show(EphemeralTip::new("other", spans("b")), &mut counts));
        // k has hit its cap: no-op, count unchanged.
        assert!(!state.show(EphemeralTip::new("k", spans("c")), &mut counts));
        assert_eq!(counts.get("k"), Some(&(1, 1)));
    }

    #[test]
    fn tick_expires_once() {
        let mut state = EphemeralTipState::default();
        let mut counts: HashMap<String, TipSeen> = HashMap::new();
        let tip = EphemeralTip::new("k", spans("hi")).with_ttl(Duration::from_millis(1));
        state.show(tip, &mut counts);
        std::thread::sleep(Duration::from_millis(5));
        assert!(state.tick(), "tick must report expiry");
        assert!(!state.is_active());
        assert!(!state.tick(), "subsequent ticks must be silent");
    }

    #[test]
    fn clear_and_clear_all() {
        let mut state = EphemeralTipState::default();
        let mut counts: HashMap<String, TipSeen> = HashMap::new();
        state.show(EphemeralTip::new("k", spans("a")), &mut counts);
        assert!(state.clear("k"));
        assert!(!state.clear("k"));
        state.show(EphemeralTip::new("k2", spans("b")), &mut counts);
        assert!(state.clear_all());
        assert!(!state.is_active());
    }

    #[test]
    fn clear_on_submit_keeps_ambient() {
        let mut state = EphemeralTipState::default();
        let mut counts: HashMap<String, TipSeen> = HashMap::new();
        state.show(EphemeralTip::new("a", spans("x")).ambient(), &mut counts);
        assert!(!state.clear_on_submit(), "ambient tips survive submit");
        assert!(state.is_active());
        // Replacing the ambient tip with a non-ambient one: submit clears the
        // non-ambient tip and the slot is empty again.
        state.show(EphemeralTip::new("b", spans("y")), &mut counts);
        assert!(state.clear_on_submit());
        assert!(!state.is_active());
    }

    #[test]
    fn plan_nudge_matches_keywords_whole_word() {
        assert!(plan_nudge_matches("plan"));
        assert!(plan_nudge_matches("Let's plan this"));
        assert!(plan_nudge_matches("PLANNING the release"));
        assert!(plan_nudge_matches("design a module"));
        assert!(plan_nudge_matches("step by step approach"));
        assert!(!plan_nudge_matches("explain this"));
        assert!(!plan_nudge_matches("planet"));
        assert!(!plan_nudge_matches("redesign"));
        assert!(!plan_nudge_matches("hello"));
        assert!(!plan_nudge_matches(""));
    }
}
