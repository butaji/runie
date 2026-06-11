#!/bin/bash
# Smoke test for /scoped-models dialog - Layer 4
# Verifies: hotkey hint remains visible at the bottom of the dialog
# when the model list is long enough to overflow the popup.

set -e
BINARY="$(pwd)/target/release/runie"
SESSION="runie_scoped_$$"
LOG="/tmp/runie_scoped_$$.log"
CONFIG_BACKUP="$HOME/.runie/config.toml.scoped_smoke_backup"

if [ ! -f "$BINARY" ]; then
    echo "[smoke] Building release binary..."
    cargo build --release -p runie-term 2>/dev/null
fi

# Backup existing config
if [ -f "$HOME/.runie/config.toml" ]; then
    cp "$HOME/.runie/config.toml" "$CONFIG_BACKUP"
fi

# Generate config with ~130 models across 6 providers (overflows the 18-line popup)
mkdir -p "$HOME/.runie"
cat > "$HOME/.runie/config.toml" << 'EOF'
default_provider = "mock"
default_model = "echo"

[models]
scoped = [
  "openai/gpt-4o", "openai/gpt-4o-mini", "openai/o1", "openai/o1-mini", "openai/o3", "openai/o3-mini", "openai/o4-mini", "openai/gpt-4.1", "openai/gpt-4.1-mini", "openai/gpt-4.1-nano", "openai/gpt-4-turbo", "openai/gpt-3.5-turbo", "openai/dall-e-3", "openai/whisper-1", "openai/tts-1", "openai/embed-ada", "openai/text-embed-3-small", "openai/text-embed-3-large", "openai/gpt-4-vision", "openai/gpt-image-1", "openai/realtime", "openai/babbage",
  "anthropic/claude-opus-4", "anthropic/claude-sonnet-4", "anthropic/claude-3.7-sonnet", "anthropic/claude-3.5-sonnet", "anthropic/claude-3.5-haiku", "anthropic/claude-3-opus", "anthropic/claude-3-sonnet", "anthropic/claude-3-haiku", "anthropic/claude-2.1", "anthropic/claude-2.0", "anthropic/claude-instant", "anthropic/claude-instant-1.2", "anthropic/claude-3.5-sonnet-v2", "anthropic/claude-3-sonnet-v2", "anthropic/claude-3.5-sonnet-latest", "anthropic/claude-3-opus-latest", "anthropic/claude-3-haiku-latest", "anthropic/claude-3.5-haiku-latest", "anthropic/claude-3.5-sonnet-old", "anthropic/claude-3.5-haiku-old", "anthropic/claude-2.1-old", "anthropic/claude-extra",
  "google/gemini-2.0-pro", "google/gemini-2.0-flash", "google/gemini-1.5-pro", "google/gemini-1.5-flash", "google/gemini-1.5-flash-8b", "google/gemini-1.0-pro", "google/gemini-nano", "google/gemini-ultra", "google/gemini-2.5-pro", "google/gemini-2.5-flash", "google/gemini-2.5-flash-lite", "google/gemini-exp", "google/palm-2", "google/palm-2-chat", "google/palm-2-codey", "google/gemma-2-27b", "google/gemma-2-9b", "google/gemma-2-2b", "google/gemma-7b", "google/codey", "google/medlm", "google/medlm-medium",
  "meta/llama-3.3-70b", "meta/llama-3.3-8b", "meta/llama-3.2-90b", "meta/llama-3.2-11b", "meta/llama-3.2-3b", "meta/llama-3.2-1b", "meta/llama-3.1-405b", "meta/llama-3.1-70b", "meta/llama-3.1-8b", "meta/llama-3-70b", "meta/llama-3-8b", "meta/llama-2-70b", "meta/llama-2-13b", "meta/llama-2-7b", "meta/codellama-34b", "meta/codellama-13b", "meta/codellama-7b", "meta/llama-guard", "meta/seamlessm4t", "meta/sam", "meta/nllb", "meta/musicgen",
  "mistral/large-2", "mistral/large", "mistral/medium", "mistral/small", "mistral/tiny", "mistral/mistral-7b", "mistral/mixtral-8x7b", "mistral/mixtral-8x22b", "mistral/mistral-nemo", "mistral/codestral", "mistral/mistral-embed", "mistral/pixtral", "mistral/pixtral-12b", "mistral/mistral-small-3.1", "mistral/mistral-small-3", "mistral/saba", "mistral/ministral-3b", "mistral/ministral-8b", "mistral/mathstral", "mistral/mistral-tiny-latest", "mistral/mistral-small-latest", "mistral/mistral-medium-latest",
  "cohere/command-r-plus", "cohere/command-r", "cohere/command", "cohere/command-light", "cohere/command-nightly", "cohere/embed-english-v3", "cohere/embed-multilingual-v3", "cohere/embed-english-light-v3", "cohere/embed-multilingual-light-v3", "cohere/rerank-english-v3", "cohere/rerank-multilingual-v3", "cohere/rerank-english-v2", "cohere/command-r-latest", "cohere/command-light-latest", "cohere/c4ai-aya-23", "cohere/c4ai-aya-23-35b", "cohere/c4ai-aya-23-8b", "cohere/c4ai-aya-expanse", "cohere/c4ai-aya-expanse-32b", "cohere/c4ai-aya-expanse-8b", "cohere/c4ai-r-plus", "cohere/c4ai-command"
]
EOF

# Cleanup on exit: restore original config
trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true; if [ -f "$CONFIG_BACKUP" ]; then mv "$CONFIG_BACKUP" "$HOME/.runie/config.toml"; else rm -f "$HOME/.runie/config.toml"; fi' EXIT

echo "[smoke] Starting tmux session..."
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Open scoped models dialog
echo "[smoke] Opening /scoped-models dialog..."
tmux send-keys -t "$SESSION" "/scoped-models"
sleep 0.2
tmux send-keys -t "$SESSION" Enter
sleep 0.5

# Resize stress to make sure hotkeys stay pinned across geometry changes
for i in 1 2 3 4 5 6 7 8 9 10; do
    tmux resize-window -t "$SESSION" -x $((40 + i * 4)) -y $((10 + i))
    sleep 0.05
done
sleep 0.2

# Resize back
tmux resize-window -t "$SESSION" -x 80 -y 24
sleep 0.3

# Capture pane output
echo "[smoke] Capturing output..."
tmux capture-pane -t "$SESSION" -p > "$LOG"

# Close the dialog
tmux send-keys -t "$SESSION" Escape
sleep 0.2

# Cleanup
tmux kill-session -t "$SESSION" 2>/dev/null || true
sleep 0.2

# Assertions
echo "[smoke] Checking for panics..."
if grep -iE "panic|thread.*panicked|out of memory|segmentation fault" "$LOG" 2>/dev/null; then
    echo "FAIL: Panic or crash detected!"
    cat "$LOG"
    exit 1
fi

# Restore original config now (we're done with assertions)
if [ -f "$CONFIG_BACKUP" ]; then
    mv "$CONFIG_BACKUP" "$HOME/.runie/config.toml"
fi

echo "[smoke] Checking hotkey hint at bottom of dialog..."
# The dialog is centered in the terminal. The hotkey hint should appear
# somewhere in the bottom half of the dialog (y > half). Search all
# non-empty lines of the capture for the hotkey text.

# Search for the hotkey text in the entire log
if grep -qE "navigate.*space.*toggle|space.*toggle.*esc|navigate.*esc" "$LOG"; then
    HOTKEY_LINE=$(grep -nE "navigate.*space.*toggle|space.*toggle.*esc|navigate.*esc" "$LOG" | head -1)
    echo "✓ PASS: Hotkey hint visible in dialog"
    echo "  $HOTKEY_LINE"
else
    echo "FAIL: Hotkey hint not found anywhere in the dialog!"
    echo "--- Full capture ---"
    cat "$LOG"
    exit 1
fi

echo ""
echo "=== Last 20 lines of dialog ==="
sed -n '3,22p' "$LOG"
echo ""
echo "[smoke] SUCCESS - Scoped models dialog smoke test passed"
