# State: Command Palette

**File:** `18_command_palette.txt`

## Description
Commands menu triggered by `/compact` or similar commands.

## Layout Structure
- **Split view:** Chat on left, command panel on right
- **Command panel:**
  - Header: "Commands ───────────────────────── [✗]"
  - Search field: "search:"
  - Section: "Session ──────────────────────────"
  - 8 menu items with keyboard shortcuts

## Menu Items
1. New Session - `Ctrl+N`
2. New Session in Work… - `Ctrl+Shift+N`
3. Switch Sessions - `/sessions`
4. Back to Home - `Ctrl+Shift+H`
5. Resume Session - `/resume`
6. Rename Session - `/rename`
7. Session Info - `/session-info`
8. Send Feedback - `/feedback`

## Interactive Elements
- `↑/↓` → Navigate
- `Enter` → Select
- `Esc` → Close

## Colors
- `┌─┐│└─┘` box frame
- `─` separators
- `[✗]` close button indicator
- `◆` bullet points for items

## Notes
- Appears as overlay panel on right side
- Input shows "compact" as partial command
- Some items show keyboard shortcuts inline
