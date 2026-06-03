# State: Welcome Screen

**File:** `01_welcome_screen.txt`

## Description
Initial landing screen when grok CLI starts.

## Layout Structure
- **Header bar:** Git branch indicator (`feat/grok-redesign`) + current directory
- **Main menu (centered):** 3 options with keyboard shortcuts
  - New worktree (ctrl-w)
  - Resume session (ctrl-s)
  - Quit (ctrl-q)
- **Tip text:** Below menu
- **Input area:** Bottom panel with prompt `❯`
- **Status bar:** Bottom right shows version `0.2.16 Beta`

## Interactive Elements
- `ctrl-w` → Opens new worktree modal
- `ctrl-s` → Opens session list
- `ctrl-q` → Quit (not functional from this screen)

## Colors
- ASCII box-drawing characters for borders
- `›` indicator for selected item
- `─` separator lines
- Branch indicator uses git symbol ``

## Notes
- This is the main entry point
- Input area shows blinking cursor `❯`
