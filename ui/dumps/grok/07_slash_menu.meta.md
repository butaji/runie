# State: Slash Menu

**File:** `07_slash_menu.txt`

## Description
Slash command menu accessible by typing `/` in input.

## Layout Structure
- **Status bar:** Memory usage indicator `9.5K / 512K`
- **Chat area:** Shows conversation history with thought blocks
- **Menu area:** 5 command options with descriptions
- **Input area:** Shows `/` with cursor

## Menu Commands
1. `/quit` - Quit the application
2. `/home` - Return to the welcome screen
3. `/new` - Start a new session
4. `/fork` - Branch current session into a peer agent
5. `/compact` - Compact conversation history

## Interactive Elements
- `Enter` → Execute selected command
- `Shift+Tab` → Change mode
- `Ctrl+.` → Shortcuts

## Colors
- `❯` indicates current selection
- `◆` marks thought/activity items
- Thought blocks show "Thought for 0.1s" timing
- `─` separators between menu items

## Notes
- Menu appears inline in chat area
- Selected item highlighted with `❯`
- Command descriptions wrap to second line
