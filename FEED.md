When finished, please enrich the feed with those. Implement each element as dedicated element

**Unified Feed with Progress Animation**

Layout: Bottom-anchored `❯` prompt. Stream above, newest at bottom.

**Event markers**

- `❯` — User input
- `◆` — Agent reasoning (animated header)
- `●` — Tool running
- `✓` — Tool complete
- `▸` — Plan step

**Progress animations by state**

| State                | Visual                                                                                                 | Mechanic                                                                                                   |
| -------------------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------- |
| **Thinking**         | `◆ Thought for 2.3s ⠋`                                                                                 | Braille spinner `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` cycles at 80ms inside the collapsible header. Timer updates live every 100ms. |
| **Tool execution**   | `● Reading src/main.rs ⠙` → `✓ Read 142 lines`                                                         | Spinner on the bullet; replaces with `✓` on completion. For >1s ops, inline mini bar: `[▓▓▓▓▓░░░░░]`.      |
| **Streaming output** | Code blocks type in character-by-character. Trailing cursor `▊` blinks at 500ms inside the block.      | Simulates real-time generation without breaking layout.                                                    |
| **Plan steps**       | `▸ 1. Analyze codebase` (dim) → `● 1. Analyze codebase ⠼` (bright + spinner) → `✓ 1. Analyze codebase` | Active step gets a left pulse border `│` drawn with `▐` or intensity shift.                                |
| **Background jobs**  | Status bar: `⬡ 3 jobs │ ⠦ building`                                                                    | Hexagon `⬡` + braille spinner. Pulsing dot `●` next to `ctrl+b` hint.                                      |
| **Interrupt / Undo** | `✗ Interrupted` fading to dim over 500ms. `↺ Rewinding...` with counter-clockwise spinner.             | One-shot animation, then collapses to a single `↺` history line.                                           |

**ASCII vocabulary**

```
⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏   braille spinner (10 frames)
▓▒░         progress gradient
█░          mini bar segments
▊           streaming cursor
⬡           background job
✗ ↺         fail / rewind
│ ▐         active step border
```

**Rule:** Only one spinner visible per viewport — the most recent active event. Completed events freeze to `✓` instantly. No motion for static history. The feed stays readable; animation only signals _current_ work.
