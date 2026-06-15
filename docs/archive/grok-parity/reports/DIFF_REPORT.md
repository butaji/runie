# Grok vs Runie UI Capture Diff Report

## State 01: Startup

**Grok:**
```
   feat/grok-redesign ~/Code/GitHub/runie/


                      New worktree                   ctrl-w
                      ─────────────────────────────────────
                      Resume session                 ctrl-s
                      ─────────────────────────────────────
                      Quit                           ctrl-q



  Tip: Press Ctrl-W to start a parallel task in its own worktree.

  ╭──────────────────────────────────────────────────────────────────────────╮
  │ ❯                                                                        │
  ╰──────────────────────────────────────────── Grok Build · always-approve ─╯

                                                          0.2.16 [stable] Beta
```

**Runie:**
```
   main ~/Code/GitHub/runie


                       New worktree                  ctrl-w

                       ─────────────────────────────────────
                       Resume session                ctrl-s

                       ─────────────────────────────────────
                       Quit                          ctrl-q



     Tip: Press Ctrl-W to start a parallel task in its own worktree.



  ╭──────────────────────────────────────────────────────────────────────────╮
  │ ❯                                                                        │
  ╰────────────────────────────────────────────────────────────────── runie ─╯
                                                                   0.1.0 Beta
```

**Differences:**

1. **Header branch/path format:**
   - Grok: `feat/grok-redesign ~/Code/GitHub/runie/` (trailing slash)
   - Runie: `main ~/Code/GitHub/runie` (no trailing slash, different branch)

2. **Token meter missing in Runie header:**
   - Grok: Has no token meter in startup state
   - Runie: Has no token meter in startup state

3. **Menu item spacing:**
   - Grok: Menu items (lines 6-10) have no blank line between items
   - Runie: Blank line (line 6) between "New worktree" and separator, blank line (line 9) between "Resume session" and separator

4. **Tip line indentation:**
   - Grok: `  Tip:` (2-space indent, line 17)
   - Runie: `     Tip:` (5-space indent, line 16)

5. **Tip line vertical position:**
   - Grok: Tip at line 17
   - Runie: Tip at line 16

6. **Status bar border format:**
   - Grok: `  ╭─...─╮` (2-space indent before ╭)
   - Runie: `   ╭─...─╮` (3-space indent before ╭)

7. **Status bar border width:**
   - Grok: `────────────────────────────────────────────` (43 dashes before text)
   - Runie: `───────────────────────────────────────────────────` (48 dashes before text)

8. **Status bar text:**
   - Grok: `Grok Build · always-approve`
   - Runie: `runie`

9. **Status bar border end:**
   - Grok: `─╯` (dash then bottom-right corner)
   - Runie: `─╯` (same)

10. **Version line indentation:**
    - Grok: `                                                          0.2.16 [stable] Beta` (58 spaces indent)
    - Runie: `                                                                   0.1.0 Beta` (67 spaces indent)

**Fix needed in Runie:**
- Remove trailing blank lines in menu section
- Change `runie` suffix to `Grok Build · always-approve`
- Adjust status bar border to 43 dashes
- Position version at 58 spaces indent with `[stable]` suffix

---

## State 02: Welcome

**Grok:**
```
   feat/grok-redesign ~/Code/GitHub/runie                     │ 7.6K / 512K │










  ╭──────────────────────────────────────────────────────────────────────────╮
  │ ❯                                                                        │
  ╰──────────────────────────────────────────── Grok Build · always-approve ─╯

  Shift+Tab:mode  │  Ctrl+.:shortcuts
```

**Runie:**
```
   main ~/Code/GitHub/runie                              │ 0 / 128.0K │


     ◆ New session started





  ╭──────────────────────────────────────────────────────────────────────────╮
  │ ❯                                                                        │
  ╰────────────────────────────────────────────────────────────────── runie ─╯
  Shift+Tab:mode  │  Ctrl+.:shortcuts
```

**Differences:**

1. **Header branch/path:**
   - Grok: `feat/grok-redesign ~/Code/GitHub/runie`
   - Runie: `main ~/Code/GitHub/runie`

2. **Token meter values:**
   - Grok: `7.6K / 512K`
   - Runie: `0 / 128.0K`

3. **Token meter spacing after path:**
   - Grok: 27 spaces between path and `│`
   - Runie: 31 spaces between path and `│`

4. **System message present in Runie only:**
   - Runie has `◆ New session started` at line 5
   - Grok has no system message in this state

5. **Status bar indent:**
   - Grok: `  Shift+Tab:mode` (2-space indent, line 23)
   - Runie: `  Shift+Tab:mode` (2-space indent, line 23) - same

6. **Status bar border width:**
   - Grok: 43 dashes before suffix
   - Runie: 48 dashes before suffix

7. **Status bar text:**
   - Grok: `Grok Build · always-approve`
   - Runie: `runie`

**Fix needed in Runie:**
- Remove `◆ New session started` system message
- Change token meter to `7.6K / 512K`
- Change suffix to `Grok Build · always-approve`
- Adjust border width to 43 dashes

---

## State 04: Thinking/Chat

**Grok (04_thinking.txt):**
```
   feat/grok-redesign ~/Code/GitHub/runie                      │ 20K / 512K │



     ❯ hello                                                         6:37 PM   █
                                                                            █
                                                                            █
     Hello! I'm Grok, ready to help with your software engineering   6:37 PM   █
     tasks in the /Users/admin/Code/GitHub/runie repo.                         █
                                                                            █
     Current branch: feat/grok-redesign (9 commits ahead of                    █
     origin).                                                                  █
                                                                            █
     What would you like to work on?                                           █
                                                                            █
     Turn completed in 2.4s.                                                   █
                                                                            █

  ╭──────────────────────────────────────────────────────────────────────────╮
  │ ❯                                                                        │
  ╰──────────────────────────────────────────── Grok Build · always-approve ─╯

  Shift+Tab:mode  │  Ctrl+.:shortcuts
```

**Runie (04_chat.txt):**
```
   main ~/Code/GitHub/runie                              │ 0 / 128.0K │


     ◆ New session started


     ❯ hello                                                          6:38 PM

                                                              0s ⇣0 [✓]


  ╭──────────────────────────────────────────────────────────────────────────╮
  │ ❯                                                                        │
  ╰────────────────────────────────────────────────────────────────── runie ─╯
  Shift+Tab:mode  │  Ctrl+.:shortcuts
```

**Differences:**

1. **Header branch/path:**
   - Grok: `feat/grok-redesign ~/Code/GitHub/runie`
   - Runie: `main ~/Code/GitHub/runie`

2. **Token meter:**
   - Grok: `20K / 512K`
   - Runie: `0 / 128.0K`

3. **Token meter spacing:**
   - Grok: 27 spaces after path
   - Runie: 31 spaces after path

4. **System message:**
   - Grok: No system message
   - Runie: `◆ New session started` (line 5)

5. **User message bullet:**
   - Grok: `❯ hello` (no trailing spaces, no bullet/checkmark after)
   - Runie: `❯ hello` (same)

6. **User message timestamp:**
   - Grok: `6:37 PM` with trailing `   █`
   - Runie: `6:38 PM` with no trailing block

7. **Assistant message bullet:**
   - Grok: `❯ Hello!` (no bullet prefix, uses ❯ like user)
   - Runie: `∘ Hello!` (different bullet - middle dot instead of right pointer)

8. **Assistant message content:**
   - Grok: "Hello! I'm Grok, ready to help with your software engineering tasks in the /Users/admin/Code/GitHub/runie repo. Current branch: feat/grok-redesign (9 commits ahead of origin). What would you like to work on? Turn completed in 2.4s."
   - Runie: "Hello! I'm a mock coding agent. How can I help you today?"

9. **Assistant message timestamp:**
   - Grok: `6:37 PM   █`
   - Runie: `6:38 PM` (no trailing block)

10. **Completion status line:**
    - Grok: No explicit completion status shown (instead "Turn completed in 2.4s" in message)
    - Runie: `0s ⇣0 [✓]` on line 15

11. **Message area right-padding:**
    - Grok: Lines have trailing `   █` block characters
    - Runie: No trailing block characters

12. **Status bar indent:**
    - Grok: `  Shift+Tab:mode` (2-space indent)
    - Runie: `  Shift+Tab:mode` (2-space indent) - same

13. **Status bar border width:**
    - Grok: 43 dashes
    - Runie: 48 dashes

14. **Status bar suffix:**
    - Grok: `Grok Build · always-approve`
    - Runie: `runie`

**Fix needed in Runie:**
- Change branch to `feat/grok-redesign`
- Change token meter to `20K / 512K`
- Remove `◆ New session started`
- Change `∘` to `❯` for assistant messages
- Add trailing `   █` block characters to message lines
- Add completion status like "Turn completed in Xs" to assistant message
- Change suffix to `Grok Build · always-approve`
- Adjust border to 43 dashes

---

## State 05: Tools

**Grok (05_tools.txt):**
```
   feat/grok-redesign ~/Code/GitHub/runie                      │ 23K / 512K │



     ❯ list files                                                    6:37 PM



  ┃  ◆ Thinking…
  ┃
  ┃  …                                                                         █
  ┃  It's a Rust project called "runie", with multiple crates under crates/,   █
  ┃  a TUI, CLI, AI components, etc. There's a target/ directory (build        █
  ┃  artifacts), docs/, harness/, etc.                                         █
                                                                            █

    ⠧ Thinking… 1.5s                                           8.0s ⇣23.2k [✗]

  ╭──────────────────────────────────────────────────────────────────────────╮
  │ ❯                                                                        │
  ╰──────────────────────────────────────────── Grok Build · always-approve ─╯

  Shift+Tab:mode  │  Ctrl+c:cancel  │  Ctrl+Enter:interject  │  Ctrl+.:
```

**Runie (05_tools.txt):**
```
  ⠴  main ~/Code/GitHub/runie                            │ 0 / 128.0K │


     ◆ New session started


     ❯ hello                                                          6:38 PM

     ∘ Hello! I'm a mock coding agent. How can I help you today?     6:38 PM

                                                              0s ⇣0 [✓]


    ⠋ Thinking                                                      [turn: 0s]
  ╭──────────────────────────────────────────────────────────────────────────╮
  │ ❯                                                                        │
  ╰────────────────────────────────────────────────────────────────── runie ─╯
  Shift+Tab:mode  │  Ctrl+c:cancel  │  Ctrl+Enter:interject  │  Ctrl+.:shortcuts
```

**Differences:**

1. **Header prefix:**
   - Grok: No prefix on line 2
   - Runie: `⠴ ` (loading spinner) prefix before ``

2. **Header branch/path:**
   - Grok: `feat/grok-redesign ~/Code/GitHub/runie`
   - Runie: `main ~/Code/GitHub/runie`

3. **Token meter:**
   - Grok: `23K / 512K`
   - Runie: `0 / 128.0K`

4. **Token meter spacing:**
   - Grok: 27 spaces after path
   - Runie: 31 spaces after path

5. **System message:**
   - Grok: No system message shown (but context implies previous messages)
   - Runie: `◆ New session started` still visible

6. **User message:**
   - Grok: `❯ list files` with `6:37 PM`
   - Runie: Shows both `❯ hello` and `∘ Hello!` messages with `6:38 PM` timestamps

7. **User message timestamp:**
   - Grok: `6:37 PM` with no trailing block on this line
   - Runie: `6:38 PM` for both messages

8. **Completion status:**
   - Grok: None shown at this point (tool is running)
   - Runie: `0s ⇣0 [✓]` shown after assistant message

9. **Thinking block - indicator line:**
   - Grok: `┃  ◆ Thinking…` (box-drawing char prefix, 2-space indent after │)
   - Runie: `   ⠋ Thinking` (3-space indent, no box-drawing char)

10. **Thinking block - indicator position:**
    - Grok: Line 9
    - Runie: Line 19

11. **Thinking block - content lines:**
    - Grok: `┃  …` on line 11, then `┃  It's a Rust project...` on lines 12-15
    - Runie: No thinking content lines shown

12. **Thinking block - right-padding:**
    - Grok: Content lines have trailing `   █`
    - Runie: N/A (no content)

13. **Thinking status line format:**
    - Grok: `  ⠧ Thinking… 1.5s                                           8.0s ⇣23.2k [✗]` (2-space indent, spinner, duration, right-padded metrics)
    - Runie: `    ⠋ Thinking                                                      [turn: 0s]` (4-space indent, spinner, no duration, bracket-wrapped turn time)

14. **Metrics format:**
    - Grok: `8.0s ⇣23.2k [✗]` (elapsed time, download arrow, size, error status)
    - Runie: `[turn: 0s]` (just turn time in brackets)

15. **Status bar indent:**
    - Grok: `  Shift+Tab:mode` (2-space indent)
    - Runie: `  Shift+Tab:mode` (2-space indent)

16. **Status bar border width:**
    - Grok: 43 dashes
    - Runie: 48 dashes

17. **Status bar suffix:**
    - Grok: `Grok Build · always-approve`
    - Runie: `runie`

18. **Status bar hints count:**
    - Grok: 4 hints (Shift+Tab:mode, Ctrl+c:cancel, Ctrl+Enter:interject, Ctrl+:)
    - Runie: 4 hints but last one has `:shortcuts` suffix

**Fix needed in Runie:**
- Add loading spinner prefix `⠴ ` to header line
- Change branch to `feat/grok-redesign`
- Change token meter to `23K / 512K`
- Remove `◆ New session started` system message
- Remove duplicate chat messages (should only show `❯ list files`)
- Change thinking indicator to use `┃  ◆ Thinking…` format with box-drawing char
- Add thinking content lines with `┃  …` and `┃  It's a Rust project...` format
- Add trailing `   █` to thinking content lines
- Change thinking status to `  ⠧ Thinking… 1.5s                                           8.0s ⇣23.2k [✗]`
- Change suffix to `Grok Build · always-approve`
- Adjust border to 43 dashes
- Remove `:shortcuts` from last hint

---

## State 06: Complete

**Grok (06_complete.txt):**
```
   feat/grok-redesign ~/Code/GitHub/runie                      │ 29K / 512K │
  ┌                                                                            ┐
  │                                                                            │
  │   ❯ list files                                                    6:37 PM  │
  │                                                                            │
  └                                                                            ┘
     ◆ List src

  ┃  ◆ Thinking…
  ┃                                                                            █
  ┃  I now have more detailed listings. First, the user query is "list         █
  ┃  files". But in the context, it seems like this is part of a larger        █
  ┃  interaction, perhaps simulating a file listing in a project.              █
                                                                            █

    ⠧ Thinking… 3.5s                                            22s ⇣29.2k [✗]

  ╭──────────────────────────────────────────────────────────────────────────╮
  │ Build anything                                                         │
  ╰──────────────────────────────────────────── Grok Build · always-approve ─╯

  Ctrl+Shift+e:expand thinking  │  Space:prompt  │  Ctrl+c:cancel  │
```

**Runie:** No complete state capture available.

---

## Summary of Required Changes by Area

### Header Format
1. Remove spinner prefix `⠴ ` from Runie header
2. Change `main` to `feat/grok-redesign`
3. Add trailing `/` to path: `~/Code/GitHub/runie/`
4. Token meter values need adjustment per state
5. Consistent spacing: 27 spaces before `│` in token meter

### Content Area
1. Remove `◆ New session started` system message
2. Change `∘` bullet to `❯` for assistant messages
3. Add trailing `   █` block characters to message lines
4. Remove duplicate messages (Runie shows both hello and list files in tools state)

### Input Bar
1. Change suffix from `runie` to `Grok Build · always-approve`
2. Adjust border width from 48 to 43 dashes
3. Change `│ ❯` to `│ ❯` (keep same)

### Status Bar
1. Change `runie` to `Grok Build · always-approve`
2. Adjust border to 43 dashes
3. Remove `:shortcuts` from last hint
4. Add `Ctrl+Shift+e:expand thinking` hint

### Activity Panel
1. Add user query box using `┌─...─┐` / `│...│` / `└─...─┘` format
2. Add `◆ List src` tool result indicator

### Thinking Blocks
1. Use `┃  ◆ Thinking…` format (box-drawing char + 2-space indent)
2. Content lines use `┃  ` prefix + content + `   █` suffix
3. Status line format: `  ⠧ Thinking… X.Xs                      Xs ⇣X.Xk [✗]`
4. Right-pad with spaces before metrics

### System Messages
1. Remove `◆ New session started`
2. Add `◆ List src` for tool results
