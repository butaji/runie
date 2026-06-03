# State: Worktree Modal

**File:** `11_worktree_modal.txt`

## Description
Modal dialog for creating a new git worktree.

## Layout Structure
- **Welcome menu:** Still visible in background
- **Modal overlay:** Centered box with:
  - Title: "New Worktree"
  - Input field: "Name (optional): █"
  - Footer hints: "enter = create   esc = cancel"

## Interactive Elements
- `Enter` → Create worktree (if name entered)
- `Esc` → Cancel and close modal

## Colors
- Box-drawing frame `╭─╮│╰─╯`
- Input cursor shown as `█`

## Notes
- Opens via `ctrl-w` from welcome screen or chat
- Optional name field - can create unnamed worktree
- Still shows welcome menu underneath
