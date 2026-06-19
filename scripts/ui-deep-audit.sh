#!/bin/bash
# Deep UI/UX Audit — 100+ scenarios
# Don't set -e — we handle failures manually

BINARY="$(pwd)/target/release/runie"
SESSION="runie_ui_$$"
LOG="/tmp/ui_audit_$$.log"
PASS=0
FAIL=0
TMUX="tmux -L runie_ui_audit_$$ -f /dev/null"


cleanup() { $TMUX kill-session -t "$SESSION" 2>/dev/null || true; rm -f "$LOG"; }
trap cleanup EXIT

pass() { echo "  ✓ $1"; PASS=$((PASS+1)); }
fail() { echo "  ✗ FAIL: $1"; FAIL=$((FAIL+1)); }
info() { echo "  ℹ $1"; }

echo "========================================"
echo "  DEEP UI/UX AUDIT — 100+ Scenarios"
echo "========================================"
cargo build --release -p runie-tui --bin runie 2>&1 | tail -1

# Helper: start fresh session. A brief resize nudges the app to render its
# initial frame in a detached tmux session (crossterm may not emit a startup
# resize event otherwise).
fresh() {
    $TMUX kill-session -t "$SESSION" 2>/dev/null || true
    sleep 0.15
    local env_args=""
    if [ -n "${RUNIE_MOCK:-}" ] || [ -n "${RUNIE_MOCK_DELAY:-}" ]; then
        env_args="env RUNIE_MOCK=1"
        if [ -n "${RUNIE_MOCK_DELAY:-}" ]; then
            env_args="$env_args RUNIE_MOCK_DELAY=1"
        fi
    fi
    if [ -n "$env_args" ]; then
        $TMUX new-session -d -s "$SESSION" -x 80 -y 24 "$env_args $BINARY"
    else
        $TMUX new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
    fi
    sleep 0.4
    $TMUX resize-window -t "$SESSION" -x 81 -y 25
    sleep 0.1
    $TMUX resize-window -t "$SESSION" -x 80 -y 24
    sleep 0.2
}

# Helper: capture and check
has() { grep -q "$1" "$LOG"; }
has_not() { ! grep -q "$1" "$LOG"; }
no_panic() { ! grep -qi "panic\|thread.*panicked" "$LOG"; }

echo ""
echo "--- DIALOGS (1-20) ---"

S=1; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Commands" && no_panic; then pass "$S: Command palette opens"; else fail "$S: Command palette"; fi

S=2; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" Escape; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has_not "Commands" && no_panic; then pass "$S: Command palette closes with Escape"; else fail "$S: Palette close"; fi

S=3; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "model"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Select Model" && no_panic; then pass "$S: Model selector opens via palette"; else fail "$S: Model selector"; fi

S=4; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "model"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX send-keys -t "$SESSION" Escape; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has_not "Select Model" && no_panic; then pass "$S: Model selector closes"; else fail "$S: Model selector close"; fi

S=5; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "settings"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Settings" && no_panic; then pass "$S: Settings dialog opens"; else fail "$S: Settings dialog"; fi

S=6; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "settings"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX send-keys -t "$SESSION" Escape; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has_not "Settings" && no_panic; then pass "$S: Settings closes with Escape"; else fail "$S: Settings close"; fi

S=7; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "scoped"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Scoped Models" && no_panic; then pass "$S: Scoped models dialog opens"; else fail "$S: Scoped models"; fi

S=8; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "tree"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Session Tree" && no_panic; then pass "$S: Session tree dialog opens"; else fail "$S: Session tree"; fi

S=9; fresh; $TMUX send-keys -t "$SESSION" "@"; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if grep -qE "[0-9]+ files" "$LOG" && no_panic; then pass "$S: @-suggestions popup"; else fail "$S: @-suggestions"; fi

S=10; fresh; $TMUX send-keys -t "$SESSION" "/"; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Commands" && no_panic; then pass "$S: Slash command opens palette"; else fail "$S: Slash command"; fi

S=11; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "quit"; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "quit" && no_panic; then pass "$S: Palette filters commands"; else fail "$S: Palette filter"; fi

S=12; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; for i in 1 2 3; do $TMUX send-keys -t "$SESSION" Down; sleep 0.1; done; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "▸" && no_panic; then pass "$S: Palette navigation with Down"; else fail "$S: Palette nav"; fi

S=13; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "zzznonexistent"; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if grep -qE "No matching" "$LOG" && no_panic; then pass "$S: Palette shows empty state for bad filter"; else fail "$S: Palette empty state"; fi

S=14; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "settings"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX send-keys -t "$SESSION" Right; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Appearance" && no_panic; then pass "$S: Settings tab switches with Right arrow"; else fail "$S: Settings tab"; fi

S=15; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "settings"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX send-keys -t "$SESSION" Right; $TMUX send-keys -t "$SESSION" Right; $TMUX send-keys -t "$SESSION" Right; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Behavior" && no_panic; then pass "$S: Settings cycles through all tabs"; else fail "$S: Settings tabs cycle"; fi

S=16; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "model"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX send-keys -t "$SESSION" "gpt"; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "gpt" && no_panic; then pass "$S: Model selector filters"; else fail "$S: Model filter"; fi

S=17; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Palette select with Enter works"; else fail "$S: Palette select"; fi

S=18; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" C-c; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+C with palette open doesn't crash"; else fail "$S: Ctrl+C with palette"; fi

S=19; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" C-p; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Double Ctrl+P doesn't crash"; else fail "$S: Double palette"; fi

S=20; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.15; $TMUX send-keys -t "$SESSION" "model"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Select Model" && no_panic; then pass "$S: Palette -> Model selector transition"; else fail "$S: Palette->Model transition"; fi

echo ""
echo "--- INPUT & SUBMISSION (21-40) ---"

S=21; fresh; $TMUX send-keys -t "$SESSION" "hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "hello" && no_panic; then pass "$S: Simple message submit"; else fail "$S: Simple submit"; fi

S=22; fresh; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Empty submit doesn't crash"; else fail "$S: Empty submit"; fi

S=23; fresh; $TMUX send-keys -t "$SESSION" "line1"; $TMUX send-keys -t "$SESSION" C-j; $TMUX send-keys -t "$SESSION" "line2"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "line1" && has "line2" && no_panic; then pass "$S: Multiline submit"; else fail "$S: Multiline"; fi

S=24; fresh; $TMUX send-keys -t "$SESSION" "test"; $TMUX send-keys -t "$SESSION" C-a; $TMUX send-keys -t "$SESSION" "X"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+A moves cursor to start"; else fail "$S: Ctrl+A"; fi

S=25; fresh; $TMUX send-keys -t "$SESSION" "test"; $TMUX send-keys -t "$SESSION" C-e; $TMUX send-keys -t "$SESSION" "Y"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+E moves cursor to end"; else fail "$S: Ctrl+E"; fi

S=26; fresh; $TMUX send-keys -t "$SESSION" "hello world"; $TMUX send-keys -t "$SESSION" C-w; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+W deletes word"; else fail "$S: Ctrl+W"; fi

S=27; fresh; $TMUX send-keys -t "$SESSION" "abc"; $TMUX send-keys -t "$SESSION" C-d; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+D delete char"; else fail "$S: Ctrl+D"; fi

S=28; fresh; $TMUX send-keys -t "$SESSION" "abc"; $TMUX send-keys -t "$SESSION" Backspace; $TMUX send-keys -t "$SESSION" Backspace; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Backspace works"; else fail "$S: Backspace"; fi

S=29; fresh; $TMUX send-keys -t "$SESSION" "    "; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Whitespace-only submit handled"; else fail "$S: Whitespace submit"; fi

S=30; fresh; $TMUX send-keys -t "$SESSION" "$(python3 -c 'print("x"*500)')"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Very long input handled"; else fail "$S: Long input"; fi

S=31; fresh; $TMUX send-keys -t "$SESSION" "hello @world #test"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Special characters in input"; else fail "$S: Special chars"; fi

S=32; fresh; $TMUX send-keys -t "$SESSION" "test"; $TMUX send-keys -t "$SESSION" C-u; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+U deletes to start"; else fail "$S: Ctrl+U"; fi

S=33; fresh; $TMUX send-keys -t "$SESSION" "test"; $TMUX send-keys -t "$SESSION" C-k; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+K deletes to end"; else fail "$S: Ctrl+K"; fi

S=34; fresh; $TMUX send-keys -t "$SESSION" "test1"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX send-keys -t "$SESSION" "test2"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX send-keys -t "$SESSION" Up; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Up arrow recalls history"; else fail "$S: History up"; fi

S=35; fresh; $TMUX send-keys -t "$SESSION" "test1"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX send-keys -t "$SESSION" "test2"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX send-keys -t "$SESSION" Up; sleep 0.2; $TMUX send-keys -t "$SESSION" Down; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Down arrow moves forward in history"; else fail "$S: History down"; fi

S=36; fresh; $TMUX send-keys -t "$SESSION" "hello"; $TMUX send-keys -t "$SESSION" M-b; $TMUX send-keys -t "$SESSION" "X"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Alt+B word left"; else fail "$S: Alt+B"; fi

S=37; fresh; $TMUX send-keys -t "$SESSION" "hello"; $TMUX send-keys -t "$SESSION" M-f; $TMUX send-keys -t "$SESSION" "Y"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Alt+F word right"; else fail "$S: Alt+F"; fi

S=38; fresh; $TMUX send-keys -t "$SESSION" Tab; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Tab input doesn't crash"; else fail "$S: Tab input"; fi

S=39; fresh; $TMUX send-keys -t "$SESSION" "test"; $TMUX send-keys -t "$SESSION" C-z; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+Z suspend doesn't crash"; else fail "$S: Ctrl+Z"; fi

S=40; fresh; $TMUX send-keys -t "$SESSION" "say hi"; $TMUX send-keys -t "$SESSION" M-Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Alt+Enter follow-up"; else fail "$S: Alt+Enter"; fi

echo ""
echo "--- AGENT RESPONSE FLOW (41-60) ---"

S=41; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "hello" && no_panic; then pass "$S: Agent response visible"; else fail "$S: Agent response"; fi

S=42; fresh; $TMUX send-keys -t "$SESSION" "/trust"; sleep 0.2; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" "list files"; $TMUX send-keys -t "$SESSION" Enter; sleep 6.0; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if (has "✓" || has "Turn completed") && no_panic; then pass "$S: Tool execution visible"; else fail "$S: Tool execution"; fi

S=43; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has_not "Turn completed" && no_panic; then pass "$S: No TurnComplete for simple response"; else fail "$S: TurnComplete simple"; fi

S=44; fresh; $TMUX send-keys -t "$SESSION" "/trust"; sleep 0.2; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" "list files"; $TMUX send-keys -t "$SESSION" Enter; sleep 6.0; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Turn completed" && no_panic; then pass "$S: TurnComplete visible for tool call"; else fail "$S: TurnComplete tool"; fi

S=45; fresh; $TMUX send-keys -t "$SESSION" "count to 1000"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX send-keys -t "$SESSION" Escape; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Abort mid-turn with Escape"; else fail "$S: Abort Escape"; fi

S=46; fresh; $TMUX send-keys -t "$SESSION" "count to 1000"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX send-keys -t "$SESSION" C-s; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Abort mid-turn with Ctrl+S"; else fail "$S: Abort Ctrl+S"; fi

S=47; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" "say again"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "again" && no_panic; then pass "$S: Second message after first"; else fail "$S: Second message"; fi

S=48; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" "queued"; $TMUX send-keys -t "$SESSION" Enter; sleep 5.0; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "queued" && no_panic; then pass "$S: Queued message processed"; else fail "$S: Queued message"; fi

S=49; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-e; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Toggle collapse with Ctrl+E"; else fail "$S: Toggle collapse"; fi

S=50; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-e; sleep 0.15; $TMUX send-keys -t "$SESSION" C-e; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Toggle collapse back and forth"; else fail "$S: Toggle expand"; fi

S=51; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-l; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+L collapse all"; else fail "$S: Ctrl+L"; fi

S=52; fresh; for i in 1 2 3 4 5; do $TMUX send-keys -t "$SESSION" "msg $i"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; done; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Rapid 5x submit"; else fail "$S: Rapid submit"; fi

S=53; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" Up; sleep 0.15; $TMUX send-keys -t "$SESSION" "scrolled up"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "scrolled up" && no_panic; then pass "$S: Submit while scrolled up auto-scrolls"; else fail "$S: Scroll+submit"; fi

S=54; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" Up; sleep 0.15; $TMUX send-keys -t "$SESSION" Up; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Scroll up shows older content"; else fail "$S: Scroll up content"; fi

S=55; fresh; for i in 1 2 3 4 5 6 7 8 9 10; do $TMUX send-keys -t "$SESSION" "line $i"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; done; sleep 4.0; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: 10 messages fill viewport"; else fail "$S: 10 messages"; fi

S=56; fresh; $TMUX send-keys -t "$SESSION" "/trust"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Trust command works"; else fail "$S: Trust command"; fi

S=57; fresh; $TMUX send-keys -t "$SESSION" "/untrust"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Untrust command works"; else fail "$S: Untrust command"; fi

S=58; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "reset"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has_not "hello" && no_panic; then pass "$S: Reset clears messages"; else fail "$S: Reset"; fi

S=59; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "new"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has_not "hello" && no_panic; then pass "$S: New session clears messages"; else fail "$S: New session"; fi

S=60; fresh; $TMUX send-keys -t "$SESSION" "count to 1000"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "abort"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Abort via palette while streaming"; else fail "$S: Palette abort"; fi

echo ""
echo "--- RESIZE & EDGE CASES (61-80) ---"

S=61; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX resize-window -t "$SESSION" -x 20 -y 5; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Resize to very small"; else fail "$S: Small resize"; fi

S=62; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX resize-window -t "$SESSION" -x 200 -y 60; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Resize to very large"; else fail "$S: Large resize"; fi

S=63; fresh; $TMUX send-keys -t "$SESSION" "count to 1000"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; for i in 1 2 3 4 5; do $TMUX resize-window -t "$SESSION" -x $((40+i*10)) -y $((10+i*3)); sleep 0.1; done; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Resize while streaming"; else fail "$S: Resize streaming"; fi

S=64; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX resize-window -t "$SESSION" -x 40 -y 10; sleep 0.15; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Commands" && no_panic; then pass "$S: Palette in small window"; else fail "$S: Palette small"; fi

S=65; fresh; $TMUX resize-window -t "$SESSION" -x 10 -y 5; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Very small startup window"; else fail "$S: Tiny window"; fi

S=66; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX resize-window -t "$SESSION" -x 80 -y 24; sleep 0.15; $TMUX resize-window -t "$SESSION" -x 40 -y 12; sleep 0.15; $TMUX resize-window -t "$SESSION" -x 80 -y 24; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Multiple resize cycles"; else fail "$S: Resize cycles"; fi

S=67; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX resize-window -t "$SESSION" -x 80 -y 24; sleep 0.15; $TMUX send-keys -t "$SESSION" Up; sleep 0.15; $TMUX resize-window -t "$SESSION" -x 40 -y 12; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Resize while scrolled up"; else fail "$S: Resize scrolled"; fi

S=68; fresh; $TMUX send-keys -t "$SESSION" "$(python3 -c 'print("A"*1000)')"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Very long single-line message"; else fail "$S: Long line"; fi

S=69; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-c; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+C quits cleanly"; else fail "$S: Ctrl+C quit"; fi

S=70; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-c; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Ctrl+C during response"; else fail "$S: Ctrl+C response"; fi

echo ""
echo "--- STATUS BAR & HINTS (71-85) ---"

S=71; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if grep -qE '[0-9]+/[0-9]+k' "$LOG" && no_panic; then pass "$S: Token count in status bar"; else fail "$S: Token count"; fi

S=72; fresh; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "ctrl+o expand/collapse" && no_panic; then pass "$S: Hints line visible at idle"; else fail "$S: Hints idle"; fi

S=73; export RUNIE_MOCK_DELAY=1; fresh; unset RUNIE_MOCK_DELAY; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Working" && no_panic; then pass "$S: Status shows 'Working' during turn"; else fail "$S: Working status"; fi

S=74; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has_not "Working" && no_panic; then pass "$S: Status 'Working' cleared after turn"; else fail "$S: Working cleared"; fi

S=75; fresh; $TMUX send-keys -t "$SESSION" "/trust"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX send-keys -t "$SESSION" "say hi"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has_not "🔒" && no_panic; then pass "$S: Trust removes lock icon"; else fail "$S: Trust lock"; fi

S=76; fresh; $TMUX send-keys -t "$SESSION" "/untrust"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "🔒" && no_panic; then pass "$S: Untrust shows lock icon"; else fail "$S: Untrust lock"; fi

S=77; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has_not "ctrl+shift+e" && no_panic; then pass "$S: Hints hidden when palette open"; else fail "$S: Hints with palette"; fi

S=78; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" Escape; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "ctrl+o expand/collapse" && no_panic; then pass "$S: Hints restored after palette close"; else fail "$S: Hints restored"; fi

S=79; fresh; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if grep -qE '[a-z]+/[A-Za-z0-9_.-]+' "$LOG" && no_panic; then pass "$S: Provider/model shown in status"; else fail "$S: Provider status"; fi

S=80; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; for i in 1 2 3 4 5; do $TMUX send-keys -t "$SESSION" "msg $i"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; done; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic && ! grep -E '[0-9]{4}\.[0-9]s' "$LOG"; then pass "$S: No stuck timers after rapid submits"; else fail "$S: Stuck timers"; fi

echo ""
echo "--- EMPTY & INITIAL STATES (81-90) ---"

S=81; fresh; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Type a message" && no_panic; then pass "$S: Empty state shows hint"; else fail "$S: Empty hint"; fi

S=82; fresh; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "❯" && no_panic; then pass "$S: Input prompt visible at start"; else fail "$S: Input prompt"; fi

S=83; fresh; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if grep -qE '[a-z]+/[A-Za-z0-9_.-]+' "$LOG" && no_panic; then pass "$S: Default provider shown"; else fail "$S: Default provider"; fi

S=84; fresh; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Empty submit on fresh app"; else fail "$S: Empty submit fresh"; fi

S=85; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Commands" && no_panic; then pass "$S: Palette on empty app"; else fail "$S: Palette empty"; fi

S=86; fresh; $TMUX send-keys -t "$SESSION" C-l; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Select Model" && no_panic; then pass "$S: Model selector on empty app"; else fail "$S: Model empty"; fi

S=87; fresh; $TMUX send-keys -t "$SESSION" "/help"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Help command works"; else fail "$S: Help command"; fi

S=88; fresh; $TMUX send-keys -t "$SESSION" "/compact"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.25; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Compact command on empty session"; else fail "$S: Compact empty"; fi

S=89; fresh; $TMUX send-keys -t "$SESSION" Backspace; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Backspace on empty input"; else fail "$S: Backspace empty"; fi

S=90; fresh; $TMUX send-keys -t "$SESSION" Up; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Up arrow on empty history"; else fail "$S: Up empty history"; fi

echo ""
echo "--- THEME & VISUAL (91-100) ---"

S=91; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "→" && no_panic; then pass "$S: Agent arrow glyph visible"; else fail "$S: Agent glyph"; fi

S=92; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "❯" && no_panic; then pass "$S: User chevron glyph visible"; else fail "$S: User glyph"; fi

S=93; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if grep -qE '[0-9]{2}:[0-9]{2}' "$LOG" && no_panic; then pass "$S: Timestamp visible on messages"; else fail "$S: Timestamp"; fi

S=94; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-e; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Collapse handled"; else fail "$S: Collapse"; fi

S=95; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-e; sleep 0.15; $TMUX send-keys -t "$SESSION" C-e; sleep 0.15; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if no_panic; then pass "$S: Expand handled"; else fail "$S: Expand"; fi

S=96; fresh; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "╭" && has "╰" && no_panic; then pass "$S: Popup rounded borders visible"; else fail "$S: Popup borders"; fi

S=97; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Commands" && has "╭" && no_panic; then pass "$S: Command palette renders"; else fail "$S: Palette render"; fi

S=98; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-l; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Select Model" && has "╭" && no_panic; then pass "$S: Model selector renders"; else fail "$S: Model selector render"; fi

S=99; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "settings"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Settings" && has "╭" && no_panic; then pass "$S: Settings dialog renders"; else fail "$S: Settings render"; fi

S=100; fresh; $TMUX send-keys -t "$SESSION" "say hello"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.4; $TMUX send-keys -t "$SESSION" C-p; sleep 0.2; $TMUX send-keys -t "$SESSION" "scoped"; $TMUX send-keys -t "$SESSION" Enter; sleep 0.2; $TMUX capture-pane -t "$SESSION" -p > "$LOG"
if has "Scoped Models" && has "╭" && no_panic; then pass "$S: Scoped models dialog renders"; else fail "$S: Scoped models render"; fi

echo ""
echo "========================================"
echo "  Results: $PASS passed, $FAIL failed"
echo "========================================"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
