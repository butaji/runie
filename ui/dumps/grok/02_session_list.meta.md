# State: Session List

**File:** `02_session_list.txt`

## Description
Resume session browser showing past conversation sessions.

## Layout Structure
- **Header bar:** Branch + directory, `[✗]` close button
- **Search bar:** `/ to search` + `All f` filter indicator
- **Session list:** Scrollable list with session titles and timestamps
  - Sessions shown with `›` indicator
  - Timestamps: "17h ago", "20h ago", "1d ago", "4d ago"
- **Footer:** Keyboard shortcuts legend

## Interactive Elements
- `Esc` → Back to welcome
- `Enter` → Select/resume session
- `^-w` → Worktree
- `↑↓` → Navigate list
- `ctrl-/` → Unknown

## Session Entries
1. "List All Files in Current Directory" - 17h ago
2. "test" - 20h ago
3. "grok" - 1d ago
4. "User Inquiry on AI Assistant Version" - 1d ago
5. "Hey!" - 20h ago
6. "List Files in Current Folder Command" - 3d ago
7. "List Files in Current Directory" - 4d ago

## Colors
- Box-drawing frame with `┌─┐│└─┘`
- Section separator line with `───`
- Session titles in `›` format
