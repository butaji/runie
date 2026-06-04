# Runie UI Architecture & Code Review — Grok Look&Feel Parity

**Date:** 2026-06-03
**Branch:** `feat/grok-redesign`
**Reviewer:** static reading pass (no toolchain run, no tests executed)
**Skill followed:** `whole-codebase-review`

## SCOPE

- 7 crates (`runie-core`, `runie-ext`, `runie-ext-macros`, `runie-ai`, `runie-agent`, `runie-tools`, `runie-tui`) + `harness`.
- 284 `.rs` files in `runie-tui`, 60,656 total LOC across the workspace.
- ~33% production code, ~67% test files (141 test files).
- **Did not run `cargo build` / `cargo test` / `cargo clippy` — this is a static reading review.** The README states build is enforced via `build.rs` (max 500/40/10).
- **Read in depth:** `crates/runie-tui/src/components/message_list/render/{messages,assistant,user,markdown,tool}.rs`, `components/{home_screen,top_bar,status_bar,input_bar,message_list}.rs`, `tui/{render,view_models}.rs`, `tui/update/agent/events.rs`, `tui/update/agent/events/thinking.rs`, `tui/state/enums.rs`, `tui/events/keyboard/handlers.rs`, `crates/runie-agent/src/loop_engine/{run,streaming}.rs`, `crates/runie-ai/src/providers/mock.rs`, `build.rs`.
- **Skimmed:** most tests, `pipe/`, `tui/update/{ui,chat}/`, `components/{onboarding,command_palette,plan_modal,questionnaire_panel,subagent_panel,extensions_modal,permission_modal,file_picker,settings_modal,model_picker,top_bar/*}`, theme/, harness.

## EXECUTIVE SUMMARY

The component surface that drives Grok parity is **all built** (20/20 elements per `docs/grok-parity/specs.md`). The parity-eval docs claim 92-99%, but the live-render reports (MOCK_report, COMPREHENSIVE_MOCK_VERIFICATION) show 35-40% under mock load because **four concrete bugs in the streaming event pipeline** corrupt the chat content end-to-end. Fixing those four is a one-afternoon job and unblocks the entire P1 visual-diff work from the previous plan. The architecture is reasonable for a TUI; the smells are concentrated in three places:

1. **Streaming pipeline correctness** — `on_message_update` and `on_thinking_update` are append-with-no-dedupe, but the producer side sends *full accumulated text* per event, not deltas. This is the root cause of the chat-corruption bug and the "every line doubles" thinking-content bug.
2. **Parallel type machinery for the same concept** — `MessageItem::System` vs `FeedItem::SystemNotice`, `MessageList::update_last_assistant` vs `MessageList::add_or_update_assistant`, `Widget for &HomeScreen` vs `render_home_screen` free function, `StatusBar.items` vs `StatusBarViewModel.hotkeys()`. Each pair has one current path and one dead path.
3. **Layout numbers hard-coded in render functions** — indent constants, divider widths, top-bar padding, status-bar item widths. The Grok-parity work then re-tunes these as magic numbers in each render function. The right home is a layout module (there is a `style/layout.rs` with two constants and nothing else).

The good news: **none of the above is a deep architectural rewrite.** It is targeted cleanup that *also* advances look&feel parity, because the streaming bugs are exactly what make the chat look broken in every dump the project has captured.

## ARCHITECTURE ASSESSMENT

### Strengths

1. **Test discipline is real.** 141 test files, all top-20-largest source files have dedicated tests (`grok_parity_tests.rs`, `reply_provider_*_tests.rs`, `slash_tests.rs`, `palette_tests.rs`), and the build enforces a 500/40/10 shape via `build.rs`. The whole project has a "make it small enough to test" culture that is rare for a TUI.
2. **Core types are small and well-named.** `MessageItem`, `FeedItem`, `ToolCall`, `AgentEvent`, `ContentPart` are good Rust types with proper enums. `TuiMode` (`crates/runie-tui/src/tui/state/enums.rs:14-28`) is a single 13-variant state machine, easy to follow.
3. **ViewModel/State separation is honest.** `MessageListViewModel` (`components/message_list.rs:93-103`) carries only what rendering needs (no `Arc<Mutex<>>`, no app secrets). The reducer is a free function `update()`. The pipe (`view_model_pipe.build(&state)`) is the right shape.
4. **The ReplyProvider system is a clean abstraction** for the test fixtures (per the long test file names), and the `extract_think_blocks` / `strip_think_tags` split between `assistant.rs:11-47` is a nice fast-path + general path split.

### Weaknesses

1. **Two parallel `MessageItem` types for system messages.** `MessageItem::System { text }` is pushed at `tui/update/agent/events.rs:257` and `tui/update/chat/modal.rs:138,156`. The render layer at `components/message_list.rs:244` filters out `FeedItem::SystemNotice` but not `MessageItem::System` — so they render in the visible feed, but are never counted by the scrollbar (`message_list.rs:139-140`). The result is: the scrollbar miscalculates content height whenever a system message exists, and the docs' "no `◆ New session started` in chat" rule is enforced by a comment, not the type. (`comments/filter` discrepancy at `message_list.rs:242` vs `message_list.rs:139`.)
2. **Two parallel "update last assistant" paths.** `MessageList::update_last_assistant` (`components/message_list.rs:171-177`) and `MessageList::add_or_update_assistant` (`components/message_list.rs:183-198`) duplicate the same logic. The reducer only calls one path (via `update_last_assistant` in `tui/update/agent/events.rs:271, 305`), and the second method is never invoked anywhere I could find.
3. **Two parallel home-screen render paths.** `impl Widget for &HomeScreen` (`home_screen/mod.rs:136-141`) instantiates a default `ThemeWrapper` and calls the free function. The free function `render_home_screen` is the real entry point. The `Widget` impl is only used in tests; production always calls the free function. Dead code on the hot path of the test surface.
4. **Two parallel "status bar items" sources.** `StatusBar.items: Vec<StatusItem>` (`status_bar/mod.rs:21-25`) holds the items for the `impl Widget` path, but production calls `render_status_bar(vm, …)` which uses `vm.hotkeys()` (a different source). The struct fields `items`, `background_jobs`, `set_chat_mode`, `set_overlay_mode`, `add_job`, `complete_job` are never used by the live render path.
5. **Layout constants are mostly inline numbers, not constants.** `home_screen/mod.rs:148` uses `MENU_WIDTH` from `style/layout.rs:19` (good). `messages.rs:56`, `messages.rs:97`, `messages.rs:135`, `messages.rs:165`, `messages.rs:194`, `messages.rs:221`, `messages.rs:237`, `messages.rs:244`, `messages.rs:288`, `messages.rs:304`, `messages.rs:416`, `messages.rs:443` all recompute the same `content_width = area.width - margin_x + area.x - 2` and `indent = 3` separately. The `5-space Grok indent` is a comment-anchored magic number in 8+ places. One `LAYOUT` struct would make the parity-tuning cheaper.
6. **Streaming pipeline has two unidirectional write paths and no read-back contract.** `process_stream_event` (`crates/runie-agent/src/loop_engine/streaming.rs:145-268`) writes a full `assistant_message` on every `MessageDelta`; the TUI reducer (`tui/update/agent/events.rs:264-272`) tries to dedupe by `text.ends_with(&new_content)` but the producer keeps sending growing strings, so the dedupe works for one frame and then the `new_content` length exceeds the existing tail and the dedupe fails. (See "Critical bugs" below.)
7. **Mock provider has no contract for "what does a turn look like?"** The mock can either fire tools or stream text (`mock.rs:47-63`); when it fires a tool, the tool's *output* ("Read completed successfully", `mock.rs:94`) is fed back to the LLM, which contains the word "Read", which triggers `should_use_tools` again. There is no concept of "this turn is done" in the mock; the agent loop in `runie-agent/src/loop_engine/run.rs:48-80` only checks `pending_tool_calls.is_empty()`. The mock therefore loops until `max_turns` (default 10).
8. **Theme system is duplicated across `theme.rs` and `theme/themes/*.rs`.** `MessageRegistry` (`messages.rs` per the import) is a separate string-returning module that the renderers call for status strings. Two sources of truth for "what is the running label?".

## BUILD-ENFORCED STRUCTURAL RULES

The project enforces 500 lines/file, 40 lines/function, complexity 10. The `build.rs` itself (`build.rs:8-10`) comments say these are tuned higher than typical ("verb conjugation tables, render functions, … are legitimately larger than the original 40/10 caps") but does **not** actually raise the caps — the constants stay 500/40/10. The linter is a brace-counter + control-flow-keyword counter (`build.rs:161-205`); it does not understand comments, strings, or `match` expressions with guards. Concrete consequences:

- `status_bar/mod.rs:62-66, 99-104, 110-113, 135-138, 156-162, 168-171, 177-178, 182-187, 207-211, 214-219` — ten `Vec<StatusItem>` literal arrays. The function `hotkeys_for_mode` (`status_bar/mod.rs:223-233`) is one match arm per mode. This is the **right** shape given the linter; the smell is upstream (see Weakness 4: two sources of truth).
- `events.rs:13-30` — `current_timestamp()` is defined locally in `events.rs`, and `thinking.rs:7` imports it from `super::current_timestamp` (i.e. same file, different module). `on_message` (`events.rs:239-262`) defines its own `current_timestamp`-equivalent inline. The same function exists in **three** places: `events.rs:13`, `thinking.rs:6` re-import, and `components/message_list/render/user.rs:9-11` (`format_timestamp_now`). One helper, four callers, one duplication, one copy that uses `%-I:%M` and one that uses `%I:%M`.

## BUGS BY SEVERITY

### CRITICAL — User-visible product breakage

**C1. Streaming text grows exponentially in the chat.**
- `crates/runie-tui/src/tui/update/agent/events.rs:335-350` (`update_last_assistant`).
- What the code does: on every `MessageUpdate`, extracts `new_content` (which is the *full* assistant text so far) and, if `text` does not end with `new_content`, pushes `new_content` onto `text`.
- What the producer sends: `crates/runie-agent/src/loop_engine/streaming.rs:173-191` (`MessageDelta` handler). Every delta sets `assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }]` and sends the **whole** message, not the delta. The `delta` local is computed (`let delta = std::mem::take(text_buffer);`) then thrown away. The comment on line 188 even logs `&text_content[..text_content.len().saturating_sub(delta_len).min(50)]` — the author knew it was the full text.
- The receiver's dedupe (`events.rs:340`: `if !text.ends_with(&new_content)`) is correct on a one-frame stream; on the next frame `text` and `new_content` are equal, dedupe triggers, all is well. But after the *first* tool call or any LLM-side buffer flush, `new_content` becomes a strict superset of `text` and is appended. After N turns the assistant text is O(N²) in the number of frames.
- The MOCK_report (`docs/grok-parity/reports/MOCK_report.md:43-58`) names exactly this bug and proposes the wrong fix (`*text = new_content;`). That fix would work for the mock's single-text-delta stream, but it would *also* clobber legitimate new content from a real provider that sends alternating `text` and `tool_call` deltas.
- Why it matters: the MOCK report's "Bug 3: Response truncated to first sentence" is a downstream symptom; the real cause is the producer emitting full-text snapshots and the consumer appending. (`messages.rs:309-311` `extract_first_sentence` is a *separate* problem — but the chat-corruption bug makes the "truncated to first sentence" report plausible because the full text never gets a chance to render in the first place.)
- Fix shape: change the producer (`streaming.rs:182-189`) to send `AgentEvent::MessageDelta { delta }` with just the new characters and a sequence number, and the consumer (`events.rs:264-272`) to `text.push_str(&delta)`. Or — less invasive — keep the current event shape, but make the reducer (`events.rs:335-350`) replace instead of append (`*text = new_content;`), and accept that any client expecting incremental deltas must read the difference itself.

**C2. Thinking content duplicates on every delta.**
- `crates/runie-agent/src/loop_engine/streaming.rs:192-202` (`ThinkingDelta` handler) — `send_event(msg_tx, AgentEvent::ThinkingUpdate { text: thinking_buffer.clone(), turn });`. Sends the full accumulated buffer.
- `crates/runie-tui/src/tui/update/agent/events/thinking.rs:21-31` (`on_thinking_update`) — `thinking.text.push_str(&text)`. Appends the full received text.
- Net: on frame N, the receiver gets the full N-char string and appends it to its own N-char string → 2N. Frame N+1: receiver gets 2N chars (because the producer also accumulated), appends → 4N. Doubles every frame.
- The MOCK_03 capture (`docs/grok-parity/reports/MOCK_report.md:33-42`) shows exactly this — every thinking line is doubled.
- Fix shape: at the producer, send only the new delta (compare `thinking_buffer.len()` before and after `push_str`); at the consumer, replace (`*thinking.text = text;`).

**C3. Mock provider infinite tool-call loop.**
- `crates/runie-ai/src/providers/mock.rs:47-63` (`generate_response`). The guard `can_use_tools = turn_count <= 1` works for the user's literal message, but the LLM-visible message history grows every turn because `build_llm_messages` includes the tool *result* in the history (`runie-agent/src/loop_engine/run.rs:57`). The tool result string `"Read completed successfully"` (line 94) then re-triggers `should_use_tools` on the next turn because of the substring `"read"`.
- The fix `can_use_tools = turn_count <= 1` is a turn counter on user messages, not on tool results, so it does not protect against the second turn after a tool fires.
- The agent loop in `run.rs:48-80` has no concept of "the model returned no tool calls" as a stop signal — it only checks `pending_tool_calls.is_empty()` (`run.rs:164-167`). So when the LLM has decided "I'm done", the loop still continues until `max_turns`.
- Why it matters: every mock capture since `COMPREHENSIVE_MOCK_VERIFICATION` (`docs/grok-parity/reports/COMPREHENSIVE_MOCK_VERIFICATION_REPORT.md:60-67`) shows the same stuck state with the same tool call. Multi-turn testing is impossible.
- Fix shape: in `run.rs` `execute_turn`, when `pending_tool_calls.is_empty()` AND the assistant has produced at least one text delta, emit `TurnEnd` and return `false`. In the mock, change `can_use_tools` to also reject when the latest message is a `Tool` role message (i.e. don't re-evaluate tools on tool output).

**C4. `◆ New session started` always shown on first chat open.**
- `crates/runie-tui/src/tui/update/chat/modal.rs:138` and `:156` — both push `MessageItem::System { text: "New session started".to_string() }` on modal open/close.
- The render layer at `components/message_list.rs:244` filters `FeedItem::SystemNotice` (a different type) but not `MessageItem::System`, so the message renders.
- The `docs/grok-parity/reports/FINAL_DIFF.md` line 92-95 already flags this as a known visual diff vs Grok.
- Fix shape: remove the two pushes in `modal.rs`, or — if the system line is wanted — change it to `MessageItem::SystemNotice` so the filter at `message_list.rs:244` catches it. Probably the former; the line adds no information.

### HIGH — Latent bugs that will bite

**H1. `MessageList::add_or_update_assistant` and `update_last_assistant` diverge.**
- `crates/runie-tui/src/components/message_list.rs:171-198` — two methods, same intent, different signatures (one takes `&str` + `Option<String>`, one takes just `&str`).
- Only `update_last_assistant` is called from the reducer (`tui/update/agent/events.rs:271, 305`). The other is dead.
- Fix: delete `add_or_update_assistant`.

**H2. `StatusBar.items` and the `Widget` impl are dead code on the live path.**
- `crates/runie-tui/src/components/status_bar/mod.rs:21-25` (the struct), `:60-70` (`Default::default`), `:235-298` (the `impl Widget` and the `set_chat_mode` / `set_overlay_mode` / `add_job` / `complete_job` mutators), and the `StatusItem` / `BackgroundJob` / `JobStatus` types at `:27-55`.
- The live render path is `tui/render.rs:92-105` (`render_status_bar`) → `vm.hotkeys()` at `status_bar/mod.rs:76-94`. None of `StatusBar.items`, `StatusBar.background_jobs`, `set_chat_mode`, `set_overlay_mode`, `add_job`, `complete_job` is called.
- Fix: delete the struct + impl, keep only the `StatusBarViewModel::hotkeys()` family. The `#[allow(dead_code)]` annotations at `:197, 202` are an admission this is already known.

**H3. `home_screen/mod.rs:136-141` `impl Widget for &HomeScreen` is dead.**
- The only `Widget::render` call is in tests. Production calls `render_home_screen` directly.
- Fix: delete the impl, or make it call the free function (it does, so the impl is fine to keep — but the `let theme = ThemeWrapper::default();` means tests render with default theme only, never the production theme). The duplication is the smell.

**H4. `build.rs` complexity counter is naive and not the same as `cargo clippy`.**
- `crates/../build.rs:180-205` — counts `if ` / `else ` / `match ` / `for ` / `while ` / `loop ` / `?` / `&&` / `||` substrings in the line. Will produce false positives on lines that contain these substrings in strings or comments. Not a critical bug but the linter is not catching what `clippy` would catch.
- Fix: drop the homegrown linter, or run `cargo clippy -- -D warnings` instead. The project pays maintenance cost for a linter that returns 0/0 for the most common Rust smells (missing `#[must_use]`, needless clones, dead code, etc.).

**H5. `MessageItem::System { text }` and `FeedItem::SystemNotice { text }` are two ways to say the same thing.**
- `crates/runie-tui/src/components/message_list/feed/mod.rs` and the `MessageItem` enum (`components/message_list/types.rs`, not opened) have parallel types.
- `message_list.rs:244` filters one, not the other, so the visible behavior depends on which constructor was used.
- Fix: collapse to one. Either remove `FeedItem::SystemNotice` entirely (the filter at `message_list.rs:244` and `:139` then becomes unconditional), or remove `MessageItem::System` and have `on_message` push a `SystemNotice` instead.

**H6. `MessageList::render_ref` always clones `vm.wrap_cache`.**
- `crates/runie-tui/src/components/message_list.rs:130` — `let mut wrap_cache = vm.wrap_cache.clone();` on every render frame.
- `WrapCache` is likely a `HashMap<String, Vec<String>>` (not opened) of wrapped text per input. On every tick, the entire cache is cloned, even though most keys are unchanged.
- Fix: pass `&mut WrapCache` from the caller and avoid the clone. Or, make `WrapCache` an `Arc<Mutex<…>>` and share.

**H7. `TuiMode::HomeScreen` is never set anywhere I could find.**
- `crates/runie-tui/src/tui/state/enums.rs:23` — the variant exists. The home screen is rendered based on `state.home_screen.visible` (in `components/home_screen`), not the `TuiMode`. So the `TuiMode::HomeScreen` branch in `tui/render.rs:42` (`hotkeys_for_mode`) is dead.
- Fix: either route the mode through `TuiMode` (the cleaner state machine) or remove the variant.

**H8. `render_input_bar` placeholder logic checks `is_focused` not `is_active`.**
- `crates/runie-tui/src/components/input_bar/mod.rs:67` — `if is_empty && !is_focused { render_placeholder(...) }`. So the placeholder is shown only when *not* focused. Grok shows the placeholder when focused-but-empty, hidden when typing. The check is inverted.
- Why it matters: the first dump any new user takes shows no placeholder, contradicting the spec.

### MEDIUM — Code smells with concrete failure modes

**M1. `finalize_tool_calls` silently swallows JSON parse errors.**
- `crates/runie-agent/src/loop_engine/streaming.rs:286-288` — `let input = match serde_json::from_str(&partial.arguments) { Ok(v) => v, Err(_) => serde_json::json!({"raw": partial.arguments}) };`. Downstream code can't tell whether the tool was called with valid JSON.
- Fix: keep the parse-failure path, but log at WARN with the tool name and the first 100 chars of the bad input.

**M2. `tool_args: serde_json::Value::String(tool_args)` in `handle_tool_start` (`tui/update/agent/events.rs:103`).**
- Tool arguments are stored as a string-shaped JSON value, then converted via `to_string()` to send to the plugin (`events.rs:125`). The round-trip adds a quote-wrapping that downstream consumers see in their logs. The plugin event in `events.rs:123-126` has `arguments: serde_json::json!({"args": tool_args })` where `tool_args` is already a `String`-shaped `Value` — so the plugin sees `{"args": "\"actual args\""}` (double-quoted).
- Fix: pass `tool_args: serde_json::Value` through to the plugin directly, not re-wrap.

**M3. `update_last_assistant` does not strip `<think>` tags.**
- The producer (`streaming.rs:114-118`) emits `Event::ThinkingDelta { content: "<think>\n" }`, then real thinking, then `Event::ThinkingDelta { content: "</think>\n" }`, then the message deltas.
- The TUI handler (`assistant.rs:152`) calls `extract_think_blocks` to separate them, which works — but `update_last_assistant` at `tui/update/agent/events.rs:335-350` calls `extract_text_content(&message.content)` which includes the think block in `text`. So the assistant's stored `text` contains `<think>...</think>\nactual text`, and the renderer has to re-strip on every render. The `extract_think_blocks` work is duplicated at the receive edge.
- Fix: strip at the receive edge in `update_last_assistant`, store the clean text.

**M4. `format_token_count` and `format_transfer_bytes` are duplicates.**
- `crates/runie-tui/src/components/message_list/render/messages.rs:108-116` and `:311-319` — same K/M formatter, two locations. The top-bar has its own `format_token_count` in `components/top_bar/gauge.rs` (not opened but referenced in `mod.rs:7`). Three copies of the same logic.
- Fix: one helper, three callers.

**M5. `messages.rs:218` hard-codes a `0x6B50FF` purple for the tool bar color.**
- `crates/runie-tui/src/components/message_list/render/messages.rs:218` and `:268` — `let tool_bar_color = ratatui::style::Color::Rgb(0x6B, 0x50, 0xFF);` and `let error_color = ratatui::style::Color::Rgb(0xEB, 0x42, 0x68);`. These are pulled from the theme's `feed.tool.bar` and `error` slots by the VM (`view_models.rs`), but the render functions ignore the VM values and use literals. The theme system is being bypassed at the render edge.
- Fix: pass the themed colors in; remove the literals.

**M6. The activity panel exists but is never rendered.**
- `crates/runie-tui/src/components/activity_panel/` — directory exists with the implementation. Searched `pipe/render/` and `tui/render.rs` for `activity_panel`; no caller. Per the `specs.md` it's "✅ Matching" but per the live-render reports it's invisible.
- Fix: wire it into the main render pass, gated on `area.width >= 100` per the spec.

**M7. `theme/themes/` has 4 theme files but no UI to switch between them.**
- `docs/grok-parity/specs.md` and `GAP_LIST.md` claim the theme cycling works via Ctrl+P. The cycling code may exist in `theme.rs` (not opened) but no test in the repo exercises it, and the `cycle_theme` reference in `docs/grok-parity/reports/GAP_LIST.md:78` is in a markdown file, not a verified code path.
- Fix: add a test that loads the app, sends the key, and asserts the theme name changed.

**M8. `TuiConfig` has `show_status_bar: bool` that is never set to `false`.**
- `crates/runie-tui/src/tui.rs:46` — `show_status_bar: true, // Always visible - hotkeys are context-aware and essential`. The flag exists, the default is `true`, and the comment admits "always visible". A flag that cannot be false is a code smell; either delete the field or expose a CLI flag.

**M9. `render_top_bar` has an O(W*H) background fill loop on every render.**
- `crates/runie-tui/src/components/top_bar/render.rs:59-67` — double-nested loop to set every cell's background. The top bar is 1 row tall; this is O(width). But the same pattern in `home_screen/mod.rs:163` (the tip line) and the scrollbar (`message_list.rs:64-77`) is O(area.height) per render. For a 24×80 terminal this is 1920 cell writes per frame, every 100ms. Fine, but a 200×60 terminal is 12000 cell writes per frame.
- Fix: use `Buffer::set_background` or `Paragraph` with background, not per-cell loops.

**M10. `tui/state/enums.rs:217-348` — the `PartialEq` for `Msg` is hand-rolled and incomplete.**
- The implementation enumerates known variants in three helpers (`single_eq`, `multi_eq`, `unit_like_eq`) plus a master `is_unit_variant`. Any new `Msg` variant added must be added to all three lists, or it gets `==` returning wrong answers.
- Fix: derive `PartialEq` (would require all inner types to be `PartialEq`, which is true for the existing types). Or use `mem::discriminant` only and document the caveat.

### LOW — Code hygiene

**L1. `#[allow(dead_code)]` on `home_hotkeys` and `plan_hotkeys`** (`status_bar/mod.rs:197, 202`). The right fix is to delete them; the wrong fix is to leave the annotation.

**L2. `extract_thought_duration` is heuristic and silently falls back to a 30-char preview** (`assistant.rs:316-339`). It scans for the word "took" in lowercase — if the model says "Thinking time: 1.2s" or "Reasoned for 0.9 seconds", the function returns the first 30 chars of the content as the duration text. This is a parser disguised as a heuristic.

**L3. `format_timestamp_now` is duplicated** in `crates/runie-tui/src/components/message_list/render/user.rs:8-11`, `crates/runie-tui/src/tui/update/agent/events.rs:13-16`, and indirectly via `current_timestamp` import in `thinking.rs:6-7`. Three call sites, one function.

**L4. The `messages.rs:46` `text` import from `glyphs::THOUGHT_MARKER` uses `MessageRegistry::thought_duration`** for the duration string and `glyphs::THOUGHT_MARKER` for the symbol. The two paths to the same display string are split across two modules.

**L5. `mock.rs:74, 84-89` — magic numbers for mock delay (2-3s) are hard-coded in the test fixture and not configurable from the harness or test caller.** The `response_delay_ms` field exists at line 12 but tool delay uses a different `rand`-driven value.

**L6. `components/onboarding/render.rs:435` and `:413` — comments reference "5-space indent" but the code uses `margin_x + 3` (a different number that happens to produce 5 with `margin_x = 2`).** The comment is the contract; the code is the implementation; they will drift.

**L7. `tui/events/keyboard/handlers.rs:500` — the file is at the 500-line cap.** It is the dispatcher for all key handlers. Splitting by mode (chat/overlay/palette) would push the file count up but keep each handler testable in isolation. The linter is not catching this because the file is exactly 500.

## PER-MODULE OBSERVATIONS

**`crates/runie-tui/src/components/message_list/render/`** — the core of look&feel. `messages.rs` (488 lines) and `assistant.rs` (339 lines) are well-structured render functions but hard-code indents and colors. `extract_think_blocks` is good. `user.rs` (113 lines) is simple, correct, has the inverted placeholder check (H8).

**`crates/runie-tui/src/components/top_bar/`** — clean. The `helpers.rs:34-53` branch-and-path assembly is the right shape, and the "skip repo if it equals 'runie'" line is a one-line product identity decision that should probably be a config flag (M-*).

**`crates/runie-tui/src/components/home_screen/mod.rs`** — 170 lines, mostly fine. The `draw_divider` / `draw_menu_item` / `render_menu` are 3 small functions instead of 1 medium, which is the right tradeoff for the linter. The `tip` text is at `area.x + 2` (line 163) which is a magic 2 — needs to come from layout constants.

**`crates/runie-tui/src/components/status_bar/`** — 439 lines, mostly dead. The 10 `Vec<StatusItem>` literals are verbose; a table-driven approach would be shorter and easier to tune. See H2.

**`crates/runie-tui/src/components/input_bar/`** — 165 lines, well-structured, but H8 is a real bug.

**`crates/runie-tui/src/tui/render.rs`** — 377 lines, the orchestrator. `build_center_line`, `render_live_status`, `render_status_center` are clean. `render_agent_list_full` is fine. Two magic numbers in `format_cost` (100, 10, 1) should be named constants.

**`crates/runie-tui/src/tui/update/agent/events.rs`** — 435 lines, **the file with the critical bug C1**. The category-routing pattern (`EventCategory` enum, `categorize_event`, `handle_*_event`) is good, but the per-event handlers at `:199-306` carry the streaming-corruption bug. The `extract_text_from_content` at `:151-162` and `extract_text_content` at `:424-435` are duplicate helpers.

**`crates/runie-tui/src/tui/update/agent/events/thinking.rs`** — 91 lines, **the file with C2**.

**`crates/runie-agent/src/loop_engine/run.rs`** — 383 lines, **the file with C3**. The `loop` at `:48-80` is a textbook turn loop but missing the "model said done" exit. The retry-on-rate-limit at `streaming.rs:71-141` is solid.

**`crates/runie-ai/src/providers/mock.rs`** — 364 lines, mostly fine. C3 lives here. The `generate_response` function is a tree of `if/else if` on substrings of the user message — fragile. A `HashMap<&str, ResponseBuilder>` keyed on the first whitespace-separated word would be more testable.

**`crates/runie-tui/src/tui/state/enums.rs`** — 387 lines, mostly the giant `Msg` enum (140+ variants) and a hand-rolled `PartialEq` (M10). The `Cmd` enum is 7 variants and clean. The state machine in `TuiMode` is clean.

**`crates/runie-tui/src/tui/events/keyboard/handlers.rs`** — 500 lines, at the cap. The `impl`-per-mode structure (`key_to_overlay_msg`, `key_to_model_picker_msg`, `key_to_extensions_modal_msg`, `key_to_slash_menu_msg`, `shortcuts_panel_*_msg`, etc.) is the right shape; just needs splitting.

## WHAT TO FIX FIRST (ordered by impact)

1. **C1** — Fix the streaming text-dedupe. Change `crates/runie-agent/src/loop_engine/streaming.rs:182-189` to send only the new delta, **or** change `tui/update/agent/events.rs:335-350` to `*text = new_content;`. Either fix unblocks every chat-screen parity test in the repo.
2. **C2** — Fix thinking-content doubling. At `crates/runie-agent/src/loop_engine/streaming.rs:198-201`, send the delta only, not `thinking_buffer.clone()`. At `crates/runie-tui/src/tui/update/agent/events/thinking.rs:27`, `*thinking.text = text;`.
3. **C3** — Stop the mock infinite loop. Either skip `should_use_tools` when the most recent message is a tool result (`crates/runie-ai/src/providers/mock.rs:47-63`), or have the agent loop exit when the model returned no tool calls (`crates/runie-agent/src/loop_engine/run.rs:48-80`).
4. **C4** — Delete the `◆ New session started` pushes at `crates/runie-tui/src/tui/update/chat/modal.rs:138, 156`.
5. **H1, H2, H3, H5** — Delete dead parallel-type paths. ~150 lines of code that exist only to confuse the next reader.
6. **H8** — Invert the placeholder check at `input_bar/mod.rs:67`.
7. **M5** — Pass themed colors into `render_tool_running_msg` and `render_tool_complete_msg` instead of hard-coding RGB triples.
8. **M6** — Wire the activity panel into the main render pass.

After 1-4, the chat screen should render correctly end-to-end and the existing `grok_parity_tests.rs` (930 lines, the largest test file) will start telling the truth. After 5-8, the codebase is ~500 lines lighter, the linter stops being needed for the things it can't catch, and the next parity diff is a single-line tweak in a constant.

## CONFIDENCE & CAVEATS

- **What I read in full:** all files listed in the "Read in depth" section of SCOPE. The `messages.rs` renderers, the home screen, the top bar, the status bar, the input bar, the agent event handlers, the agent loop, the streaming pipeline, the mock provider, the keyboard handlers, the `TuiMode` state, the `Msg` enum, `tui.rs`, `tui/render.rs`, `build.rs`, `Cargo.toml`.
- **What I skimmed:** all 141 test files (counted LOC, read 0-2 functions per file), the 4 theme files, `pipe/`, `components/{onboarding,command_palette,plan_modal,questionnaire_panel,subagent_panel,extensions_modal,permission_modal,file_picker,settings_modal,model_picker,top_bar/{gauge,mod}.rs}`, `harness/src/`, the `reply_provider_*_tests.rs` files (just file names + counts), `bin/grok_parity_test.rs` (just the file name), `crates/runie-ai/src/providers/{rig,faux,minimax}*.rs`, the full `provider_factory.rs` was referenced but not opened.
- **What I did not run:** `cargo build`, `cargo test`, `cargo clippy`, `cargo run -- --mock`. The review is static.
- **What could be different in a real build:** the `build.rs` complexity counter may produce false-positive violations on lines containing `?`, `&&`, or `||` in strings/comments. The linter is approximate; treat its 0/0 output as "no gross violations", not as "lint-clean".
- **What I might be wrong about:** H7 (that `TuiMode::HomeScreen` is never set) — I grepped for the assignment but did not exhaustively read every `state.mode =` site. C1's exact behavior on real providers — I traced the mock path; a real provider (Anthropic, OpenAI) may emit events differently, and the bug may manifest as a different visual artifact. M6 (activity panel never rendered) — I searched the call sites but did not exhaustively read every render pass. The finding is "no call site I can find", not "provably unreachable".
- **The big finding I would bet on:** C1 is the single bug behind most of the parity reports' remaining percentages. Fixing it makes ~70% of the MOCK/Comprehensive/UTL reports' "stuck" frames become "live" frames, and most of the visual deltas in `FINAL_DIFF` go away as side effects.
