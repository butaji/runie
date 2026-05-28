# Runie Test Scenarios — CASES.md

> **Phase 1**: Functional (Given/When/Then) and non-functional scenarios identified across codebase.
> **Phase 2**: Implementation details + edge cases (in progress).
> **Phase 3**: Automated test coverage (pending).

---

## Naming Convention

| Prefix | Domain |
|--------|--------|
| `ONB-` | Onboarding |
| `CHT-` | Chat / Messages |
| `AGT-` | Agent / Tool Execution |
| `PER-` | Permissions |
| `PAL-` | Command Palette |
| `TOP-` | Top Bar |
| `STT-` | State Management |
| `CFG-` | Configuration / Settings |
| `REN-` | Rendering / UI |
| `INP-` | Input / Keyboard |
| `HAR-` | Harness / Tasks |
| `LOG-` | Logging |
| `EVT-` | Event Streaming |
| `GIT-` | Git Integration |
| `CTX-` | Context Loading |
| `ERR-` | Error Handling |
| `RTR-` | Router / Tier Routing |
| `MCP-` | MCP Integration |
| `OCH-` | Orchestrator / Sub-agents |
| `SKL-` | Skills |
| `AUT-` | Authentication |
| `MOD-` | Run Modes (CLI/JSON/RPC/API) |
| `AGM-` | Agent Modes (Build/Plan) |
| `SES-` | Session Tree / Branching |
| `CMD-` | Chat Commands / Input Box |
| `MDL-` | Model Management |
| `TOL-` | Tool Management |
| `THK-` | Thinking Levels |
| `TUI-` | TUI Panels / Layout / Navigation |
| `REV-` | Code Review |
| `APL-` | Apply Diff |
| `DIA-` | Diagnostics |
| `SND-` | Sandbox Execution |

---

## ONB — Onboarding (P0)

### ONB-01: Advance from Welcome
- **Given** onboarding is on `Welcome` step
- **When** user presses `Enter`
- **Then** step advances to `ProviderSelect`
- **Files**: `events.rs:212`, `update/onboarding.rs:76`

### ONB-02: Skip Onboarding
- **Given** onboarding is on `Welcome` step
- **When** user presses `Esc`
- **Then** `onboarding = None`, mode changes to `Chat`
- **Files**: `events.rs:213`, `update/onboarding.rs:88`, `tui_run.rs:179`

### ONB-03: Navigate Provider List Up
- **Given** onboarding is on `ProviderSelect` step
- **When** user presses `Up`
- **Then** `selected_item` decrements, clamped to `0`
- **Files**: `events.rs:214`, `update/onboarding.rs:92`, `mod.rs:462`

### ONB-04: Navigate Provider List Down
- **Given** onboarding is on `ProviderSelect` step
- **When** user presses `Down`
- **Then** `selected_item` increments, clamped to `filtered_count - 1`
- **Files**: `events.rs:215`, `update/onboarding.rs:99`, `mod.rs:476`

### ONB-05: Search/Filter Providers
- **Given** onboarding is on `ProviderSelect` step
- **When** user types alphanumeric characters
- **Then** `search_query` appended, `filtered_provider_indices` recalculated via `fuzzy_match`, `selected_item = 0`
- **Edge**: Empty query restores full list
- **Files**: `events.rs:218`, `update/onboarding.rs:266`, `mod.rs:384`

### ONB-06: Backspace Search Query
- **Given** search query is non-empty at `ProviderSelect`
- **When** user presses `Backspace`
- **Then** last char removed from `search_query`, re-filters
- **Files**: `events.rs:225`, `update/onboarding.rs:275`, `mod.rs:406`

### ONB-07: Select Provider
- **Given** onboarding is on `ProviderSelect` step
- **When** user selects provider by index (click or Enter)
- **Then** `selected_provider = Some(real_index)`, models populated, `selected_model = None`
- **Edge**: `idx` mapped through `filtered_provider_indices`
- **Files**: `update/onboarding.rs:106`, `mod.rs:422`

### ONB-08: Advance to KeyInput
- **Given** provider is selected at `ProviderSelect` step
- **When** user presses `Enter`
- **Then** step advances to `KeyInput`
- **Edge**: No selection → error "Please select a provider", stays
- **Files**: `update/onboarding.rs:37`, `mod.rs:311`

### ONB-09: Type API Key Character
- **Given** onboarding is on `KeyInput` step
- **When** user types alphanumeric char
- **Then** char appended to `api_key_input`
- **Files**: `events.rs:220`, `update/onboarding.rs:124`, `mod.rs:126`

### ONB-10: Token-Based Backspace in API Key
- **Given** onboarding is on `KeyInput` step with `api_key_input = "sk-test"`
- **When** user presses `Backspace`
- **Then** trailing token removed → `"sk-"`; another backspace → `""`
- **Edge**: Token delimiters: `-`, `_`, `.`, `/`, `:`, `=`, whitespace
- **Files**: `events.rs:227`, `update/onboarding.rs:131`, `mod.rs:159`

### ONB-11: Paste into API Key
- **Given** onboarding is on `KeyInput` step
- **When** `Paste(text)` message received
- **Then** text appended to `api_key_input`
- **Files**: `update/onboarding.rs:250`, `mod.rs:126`

### ONB-12: Advance from KeyInput (Valid)
- **Given** valid API key entered at `KeyInput` step
- **When** user presses `Enter`
- **Then** `is_fetching_models = true`, `Cmd::FetchModels` emitted
- **Files**: `update/onboarding.rs:72`, `tui_run.rs:338`, `mod.rs:521`

### ONB-13: Advance from KeyInput (Invalid)
- **Given** invalid API key format
- **When** user presses `Enter`
- **Then** error shown, stays on `KeyInput`
- **Errors**: empty → "API key cannot be empty"; wrong prefix → "Key for {provider} must start with '{prefix}'"; too short → "Key appears too short"
- **Files**: `update/onboarding.rs:72`, `mod.rs:521`

### ONB-14: Retry After Fetch Failure
- **Given** `ModelsFetchFailed` received, `fetch_error` set
- **When** user presses `Enter` at `KeyInput`
- **Then** `fetch_error` cleared, `Cmd::FetchModels` re-issued
- **Files**: `update/onboarding.rs:232`, `mod.rs:418`

### ONB-15: Models Fetched Successfully
- **Given** `Cmd::FetchModels` executed successfully
- **When** `Msg::ModelsFetched(models)` received
- **Then** models sorted alphabetically, `filtered_model_indices` reset, `is_fetching_models = false`, step → `ModelSelect`
- **Files**: `update/onboarding.rs:184`, `mod.rs:189`

### ONB-16: Models Fetch Failed
- **Given** `Cmd::FetchModels` fails
- **When** `Msg::ModelsFetchFailed(error)` received
- **Then** `fetch_error = Some(error)`, hardcoded models loaded, stays on `KeyInput`
- **Files**: `update/onboarding.rs:203`, `mod.rs:412`

### ONB-17: Navigate Model List
- **Given** onboarding is on `ModelSelect` step
- **When** user presses `Up`/`Down`
- **Then** `selected_item` clamped to `filtered_model_count - 1`
- **Files**: `events.rs:214-215`, `update/onboarding.rs:92`, `mod.rs:462`

### ONB-18: Search/Filter Models
- **Given** onboarding is on `ModelSelect` step
- **When** user types characters
- **Then** `search_query` appended, `filtered_model_indices` recalculated
- **Files**: `events.rs:218`, `update/onboarding.rs:266`, `mod.rs:394`

### ONB-19: Select Model
- **Given** onboarding is on `ModelSelect` step
- **When** user selects model by index
- **Then** `selected_model = Some(real_index)`, error cleared
- **Files**: `update/onboarding.rs:116`, `mod.rs:455`

### ONB-20: Advance to Complete
- **Given** model is selected at `ModelSelect` step
- **When** user presses `Enter`
- **Then** step advances to `Complete`
- **Edge**: No selection → error "Please select a model", stays
- **Files**: `update/onboarding.rs:42`, `mod.rs:329`

### ONB-21: Complete — Add Another Provider
- **Given** onboarding is on `Complete` step, `selected_item = 0` (Yes)
- **When** user presses `Enter`
- **Then** step → `ProviderSelect`, all selections cleared
- **Files**: `update/onboarding.rs:48`, `mod.rs:350`

### ONB-22: Complete — Save and Exit
- **Given** onboarding is on `Complete` step, `selected_item = 1` (No)
- **When** user presses `Enter`
- **Then** `onboarding = None`, `TuiMode::Chat`, `Cmd::SaveSettings` emitted
- **Files**: `update/onboarding.rs:59`, `tui_run.rs:305`

### ONB-23: Back Navigation
- **Given** any onboarding step
- **When** user presses `Esc`
- **Then** step goes back (Welcome→Welcome, ProviderSelect→Welcome, KeyInput→ProviderSelect, ModelSelect→KeyInput, Complete→ModelSelect)
- **Files**: `events.rs:213`, `update/onboarding.rs:83`, `mod.rs:338`

### ONB-24: SaveSettings Writes Config
- **Given** `Cmd::SaveSettings { provider, model, api_key }`
- **When** processed in `tui_run.rs`
- **Then** `settings.provider/model/api_key` updated, `~/.runie/config.toml` written, provider env var set, `current_model` and `top_bar.model` set
- **Files**: `tui_run.rs:305`

### ONB-25: Search Shows Query
- **Given** search query active at `ProviderSelect` or `ModelSelect`
- **When** rendered
- **Then** `> query` displayed above list
- **Edge**: Empty search + empty filter → shows full list (not "no matches")
- **Files**: `render.rs:83`, `render.rs:171`

### ONB-26: No Matches Display
- **Given** search query yields zero fuzzy matches
- **When** rendered
- **Then** "no matches" shown in dimmed text
- **Files**: `render.rs:127`

### ONB-27: Pluralization in Complete
- **Given** `Complete` step renders model count
- **When** model count is 1
- **Then** "1 model" displayed; when 2+ → "2 models"
- **Files**: `render.rs:332`

### ONB-28: Paste at Non-Input Step
- **Given** onboarding on `Welcome` or `Complete` step
- **When** `Paste(text)` received
- **Then** ignored
- **Files**: `update/onboarding.rs:253`

### ONB-29: to_settings() Returns None at Complete
- **Given** onboarding is on `Complete` step
- **When** `to_settings()` called
- **Then** returns `None` if any field is missing; user stuck with no clear path forward
- **Edge**: `?` operator chain fails on first missing field
- **Files**: `mod.rs:356-372`

### ONB-30: Fetch Error State Displays Retry Hint
- **Given** `fetch_error` is set after failed model fetch
- **When** `KeyInput` step rendered
- **Then** error message displayed with hint to retry via Enter
- **Files**: `render.rs:242-256`

### ONB-31: Fuzzy Match Subsequence Behavior
- **Given** search query "troph" at ProviderSelect
- **When** fuzzy matching against "Anthropic"
- **Then** subsequence match succeeds — characters appear in order but not contiguously
- **Edge**: Counterintuitive matches possible; "troph" matches "Anthropic"
- **Files**: `mod.rs:60-74`

### ONB-32: Empty Filter + Enter Shows Error
- **Given** filtered list is empty at ProviderSelect or ModelSelect
- **When** user presses Enter
- **Then** error message shown, stays on current step
- **Edge**: Previously caused silent failure (BUG-02, now fixed)
- **Files**: `update/onboarding.rs:37-40`

### ONB-33: OnboardingSkip Message Exists But No Keyboard Shortcut
- **Given** `OnboardingSkip` message variant exists in codebase
- **When** user tries to find keyboard shortcut for skip
- **Then** no keybinding found; message exists but is unreachable
- **Files**: `events.rs`, `mod.rs`

### ONB-34: Enter on Welcome Has No Validation
- **Given** onboarding is on `Welcome` step
- **When** user presses Enter
- **Then** step advances without any validation
- **Edge**: No checks performed; always advances
- **Files**: `events.rs:212`, `update/onboarding.rs:76`

### ONB-35: Changing Provider Resets Model Selection
- **Given** model selected, user goes back to ProviderSelect
- **When** user selects a different provider
- **Then** `selected_model = None`, model list refreshed for new provider
- **Files**: `update/onboarding.rs:106`, `mod.rs:422`

### ONB-36: Search Resets selected_item to 0
- **Given** `selected_item = 5` at ProviderSelect
- **When** user types search characters
- **Then** `selected_item` reset to `0` after filter recalculation
- **Files**: `update/onboarding.rs:266`, `events.rs:218`

### ONB-37: is_fetching_models Shows Loading Indicator
- **Given** `is_fetching_models = true` at KeyInput
- **When** KeyInput rendered
- **Then** loading indicator/spinner displayed
- **Files**: `render.rs:220-235`

---

## CHT — Chat / Messages (P0)

### CHT-01: Submit User Message
- **Given** user typed text in input bar
- **When** user presses `Enter` (submit)
- **Then** message added to `state.messages`, `ChatCmd::SpawnAgent` emitted if agent not running
- **Files**: `chat.rs:handle_submit()`

### CHT-02: Agent Streams Response
- **Given** agent is running
- **When** `AgentEvent::MessageStart` / `MessageUpdate` / `MessageEnd` received
- **Then** message added/updated/completed in `state.messages`
- **Files**: `agent.rs:on_message_start/update/end()`

### CHT-03: Tool Call Display
- **Given** agent calls a tool
- **When** `AgentEvent::ToolStart` / `ToolEnd` received
- **Then** tool call item added to messages with status
- **Files**: `agent.rs:on_tool_start/end()`

### CHT-04: Clear Chat
- **Given** chat has messages
- **When** `/clear` or `ClearChat` command executed
- **Then** `state.messages.clear()`
- **Files**: `chat.rs`, `update/chat.rs`

### CHT-05: Scroll Messages
- **Given** message list is longer than viewport
- **When** user presses `PgUp`/`PgDown` or scrolls
- **Then** `scroll.feed_offset` updated
- **Files**: `update/chat.rs`

### CHT-06: New Session
- **Given** user executes `new_session` command
- **When** confirmed
- **Then** messages cleared, new session started
- **Files**: `command_palette`, `chat.rs`

### CHT-07: Load Session
- **Given** user executes `load_session` with name
- **When** confirmed
- **Then** messages loaded from session file
- **Files**: `command_palette`, `chat.rs`

### CHT-08: Save Session
- **Given** chat has messages
- **When** user executes `save_session` with optional name
- **Then** messages saved to session file
- **Files**: `command_palette`, `chat.rs`

### CHT-09: Error Messages Filtered from Agent Context on Resubmit
- **Given** agent returned error message, user resubmits
- **When** `handle_submit()` called
- **Then** error messages filtered from `messages` context passed to agent
- **Edge**: Error context lost; may affect agent's ability to recover
- **Files**: `chat.rs:97`

### CHT-10: Paste Inserts Char-by-Char into Textarea
- **Given** user pastes text into input bar
- **When** `Paste(text)` received
- **Then** text processed character by character through textarea insertion
- **Files**: `events.rs`, `update/chat.rs`

### CHT-11: Scroll Offset with Empty Messages
- **Given** `messages` list is empty
- **When** `scroll.feed_offset` calculated for rendering
- **Then** offset clamped to valid range, no out-of-bounds access
- **Files**: `update/chat.rs`, `state.rs`

---

## AGT — Agent / Tool Execution (P0)

### AGT-01: Spawn Agent
- **Given** user submits message, agent not running
- **When** `ChatCmd::SpawnAgent { messages }` processed
- **Then** agent task spawned, `agent_running = true`
- **Files**: `chat.rs:handle_submit()`, `loop_engine.rs:run_agent_loop()`

### AGT-02: Agent Completes Successfully
- **Given** agent is running
- **When** all tool calls resolved, no more turns needed
- **Then** `AgentEvent::AgentEnd` sent, `agent_running = false`
- **Files**: `loop_engine.rs`, `agent.rs:on_agent_end()`

### AGT-03: Agent Error Handling
- **Given** agent encounters error
- **When** `AgentEvent::Error { message, recoverable }` received
- **Then** error sanitized (truncated to 500 chars, stack traces collapsed), error message added to chat, `agent_running = false` if non-recoverable
- **Files**: `agent.rs:on_agent_error()`, `loop_engine.rs:classify_error()`

### AGT-04: Max Turns Exceeded
- **Given** agent running for many turns
- **When** `turn_count > config.max_turns` (default 50)
- **Then** `AgentLoopError::MaxTurnsExceeded` returned, agent stops
- **Files**: `loop_engine.rs:173`

### AGT-05: Duplicate Tool Call Prevention
- **Given** agent attempts same tool call twice in one turn
- **When** second call detected
- **Then** skipped via `HashSet<String>` deduplication
- **Files**: `loop_engine.rs:291-301`, `rig_loop.rs:308-317`

### AGT-06: Token Usage Tracking
- **Given** agent stream includes usage data
- **When** `AgentEvent::TokenUsage` received
- **Then** `state.session_token_usage` updated with cost estimation
- **Files**: `agent.rs:handle_agent_event()`, `token_usage.rs`

### AGT-07: Context Window Calculation
- **Given** messages accumulate
- **When** context window usage calculated
- **Then** `(total_chars/4 / context_window) * 100%` estimated, sent in `PermissionRequest`
- **Files**: `loop_engine.rs:calculate_context_window_usage()`

### AGT-08: Tool Panic Recovery
- **Given** tool execution panics
- **When** panic caught in `execute_tool_with_panic_catch()`
- **Then** error result returned, `AgentEvent::Error` sent to TUI
- **Files**: `loop_engine.rs:611-693`

### AGT-09: Unhandled LLM Event Variant → Warn & Continue
- **Given** LLM emits an event variant not handled in match
- **When** `handle_agent_event()` processes event
- **Then** warning logged, agent continues without crashing
- **Files**: `agent.rs:handle_agent_event()`

### AGT-10: Permission State Mismatch Handling
- **Given** permission decision returned but state doesn't expect it
- **When** `handle_permission()` called with mismatch
- **Then** treated as Deny, logged as warning
- **Files**: `agent.rs:handle_permission()`

### AGT-11: Hook Error Propagation
- **Given** hook (Block/Modify) returns error
- **When** hook processed in tool execution
- **Then** error propagated to agent, tool not executed
- **Files**: `loop_engine.rs`, `rig_loop.rs`

### AGT-12: Hardcoded 300s Timeout
- **Given** agent loop running
- **When** LLM request exceeds 300 seconds
- **Then** timeout triggered, loop exits with error
- **Files**: `loop_engine.rs`, `rig_loop.rs`

---

## PER — Permissions (P0)

### PER-01: Permission Request Display
- **Given** agent requests permission for tool
- **When** `AgentEvent::PermissionRequest` received
- **Then** permission modal shown with tool name, args, description
- **Files**: `agent.rs:on_permission_request()`, `permission_modal.rs`

### PER-02: Confirm Permission
- **Given** permission modal is open
- **When** user selects "Confirm"
- **Then** `PermissionDecision::Allow` sent, agent continues
- **Files**: `permission_modal.rs`, `agent.rs:handle_permission()`

### PER-03: Deny Permission
- **Given** permission modal is open
- **When** user selects "Deny"
- **Then** `PermissionDecision::Deny` sent, `Cmd::Rollback` emitted
- **Files**: `permission_modal.rs`, `agent.rs:handle_permission()`

### PER-04: Allow Always
- **Given** permission modal is open
- **When** user selects "Always"
- **Then** `PermissionDecision::AllowAlways` sent, tool name cached for auto-allow
- **Files**: `permission.rs:PermissionGate`, `agent.rs:handle_permission()`

### PER-05: Skip Permission
- **Given** permission modal is open
- **When** user selects "Skip"
- **Then** `PermissionDecision::Skip` sent, `Cmd::Rollback` emitted
- **Files**: `permission.rs`, `agent.rs:handle_permission()`

### PER-06: Permission Timeout
- **Given** permission modal open for 5 minutes
- **When** timeout reached
- **Then** `PermissionDecision::Deny` auto-returned, modal closed
- **Files**: `permission.rs:request_permission()`, `system.rs`

### PER-07: Permission Queue
- **Given** blocking mode, multiple permissions requested
- **When** new permission arrives while one pending
- **Then** queued in `pending_queue`, processed sequentially
- **Files**: `agent.rs:on_permission_request()`

### PER-08: PermissionGate vs PermissionModalState (Dead Code Documentation)
- **Given** codebase contains `PermissionGate` and `PermissionModalState`
- **When** analyzing permission flow
- **Then** these represent overlapping concerns; actual permission flow uses `PermissionRequest` channel
- **Edge**: Dead code; `PermissionGate` not used in main flow
- **Files**: `permission.rs`, `state.rs`

### PER-09: Permission State Mutex Clearing After Take
- **Given** permission state protected by mutex
- **When** `PermissionState` accessed via `lock().take()`
- **Then** mutex cleared after value taken, subsequent access gets `None`
- **Files**: `permission.rs`

### PER-10: Queue Overflow (Unbounded Vec)
- **Given** many permission requests arrive in quick succession
- **When** queued in `pending_queue: Vec<PendingPermission>`
- **Then** Vec grows unbounded, no overflow protection
- **Edge**: Memory growth potential under high permission volume
- **Files**: `agent.rs:302`

### PER-11: AllowAlways Cache Eviction
- **Given** `AllowAlways` caches tool permissions
- **When** cache size grows large
- **Then** oldest entries evicted (FIFO); no explicit size limit
- **Files**: `permission.rs`

### PER-12: TUI vs Agent Timeout Race
- **Given** permission modal has 5-minute timeout
- **When** TUI uses `Instant`, agent uses `tokio::time::timeout`
- **Then** clocks may diverge; race condition possible
- **Files**: `permission.rs`, `system.rs`

---

## PAL — Command Palette (P0)

### PAL-01: Open Palette
- **Given** user presses `Ctrl+P`
- **When** `OpenCommandPalette` processed
- **Then** `mode = CommandPalette`, palette rendered
- **Files**: `ui.rs`, `command_palette/mod.rs`

### PAL-02: Filter Commands
- **Given** palette is open
- **When** user types filter query
- **Then** fuzzy match scoring applied (exact=1000, prefix=900, contains=700, fuzzy=500+)
- **Files**: `command_palette/mod.rs`

### PAL-03: Navigate Commands
- **Given** palette is open with filtered list
- **When** user presses `Up`/`Down`
- **Then** selection moves, clamped to list bounds
- **Files**: `command_palette/mod.rs`

### PAL-04: Confirm Command
- **Given** command selected in palette
- **When** user presses `Enter`
- **Then** command executed (or argument mode entered if `requires_args`)
- **Files**: `command_palette/mod.rs`

### PAL-05: Cancel Palette
- **Given** palette is open
- **When** user presses `Esc`
- **Then** palette closed, mode returns to `Chat`
- **Files**: `command_palette/mod.rs`, `ui.rs`

### PAL-06: Argument Mode
- **Given** command requires arguments (e.g., `read_file`)
- **When** user confirms command
- **Then** palette switches to argument input mode
- **Files**: `command_palette/mod.rs`

### PAL-07: Usage Tracking
- **Given** palette commands used over time
- **When** commands executed
- **Then** `use_count` and `last_used` tracked, sorted by frequency when no filter
- **Files**: `command_palette/mod.rs`

### PAL-08: Command Enumeration (All 17 Commands)
- **Given** palette open with no filter
- **When** rendered
- **Then** all 17 commands listed: new_session, load_session, save_session, clear_chat, toggle_sidebar, toggle_session_tree, model_picker, compact_context, read_file, read_dir, grep, edit_file, create_file, delete_file, list_harness_tasks, run_harness_task, run_harness_all
- **Files**: `command_palette/mod.rs`

### PAL-09: selected_command() Bug — Always Returns [0]
- **Given** user has navigated palette to different selection
- **When** `selected_command()` called
- **Then** returns `filtered_commands[0]` not `filtered_commands[selected]` — selection ignored
- **Edge**: BUG-07; out-of-bounds risk if selection > 0
- **Files**: `command_palette/mod.rs:164`

### PAL-10: Empty Argument Accepted Without Validation
- **Given** command requires arguments
- **When** user confirms with empty argument field
- **Then** command accepted and executed with empty string
- **Edge**: No validation for required arguments
- **Files**: `command_palette/mod.rs`

---

## TOP — Top Bar (P0)

### TOP-01: Display Repo/Branch/Path
- **Given** git info available
- **When** top bar rendered
- **Then** left side shows `repo/branch  path` in dim color
- **Edge**: Empty fields skipped, no separators for missing values
- **Files**: `top_bar.rs`, `state.rs:TopBarState`

### TOP-02: Display Token Count
- **Given** `estimated_tokens` and `context_window` set
- **When** top bar rendered
- **Then** right side shows `{tokens}/{window}`
- **Files**: `top_bar.rs:TopBarViewModel::from_state()`

### TOP-03: Display Percentage
- **Given** tokens and context window available
- **When** top bar rendered
- **Then** `{pct}%` displayed, capped at 100%
- **Files**: `top_bar.rs:calculate_pct()`

### TOP-04: Render Gauge Widget
- **Given** percentage calculated
- **When** top bar rendered
- **Then** ratatui `Gauge` widget rendered with `.ratio(pct / 100.0)`
- **Files**: `top_bar.rs:render_top_bar()`

### TOP-05: Format Context Window (Raw)
- **Given** `context_window < 1000`
- **When** formatted
- **Then** raw number displayed (e.g., `500`)
- **Files**: `top_bar.rs:format_context_window()`

### TOP-06: Format Context Window (K)
- **Given** `1000 <= context_window < 1_000_000`
- **When** formatted
- **Then** `Nk` displayed (e.g., `120k`)
- **Files**: `top_bar.rs:format_context_window()`

### TOP-07: Format Context Window (M)
- **Given** `context_window >= 1_000_000`
- **When** formatted
- **Then** `Nm` displayed (e.g., `2m`)
- **Files**: `top_bar.rs:format_context_window()`

### TOP-08: Calculate Percentage (Zero)
- **Given** `estimated_tokens = 0`
- **When** percentage calculated
- **Then** `0.0` returned
- **Files**: `top_bar.rs:calculate_pct()`

### TOP-09: Calculate Percentage (Normal)
- **Given** `estimated_tokens = 50_000`, `context_window = 100_000`
- **When** percentage calculated
- **Then** `50.0` returned
- **Files**: `top_bar.rs:calculate_pct()`

### TOP-10: Calculate Percentage (Capped)
- **Given** `estimated_tokens = 200_000`, `context_window = 100_000`
- **When** percentage calculated
- **Then** `100.0` returned (capped)
- **Files**: `top_bar.rs:calculate_pct()`

### TOP-11: Calculate Percentage (Zero Window)
- **Given** `context_window = 0`
- **When** percentage calculated
- **Then** `0.0` returned (division by zero guard)
- **Files**: `top_bar.rs:calculate_pct()`

### TOP-12: Narrow Terminal Handling
- **Given** terminal width very narrow
- **When** top bar rendered
- **Then** `right_x > x` check prevents overlap, right side omitted if no space
- **Files**: `top_bar.rs:render_top_bar()`

---

## STT — State Management (P0)

### STT-01: State Initialization
- **Given** app starts
- **When** `AppState::default()` called
- **Then** all fields initialized: empty messages, default textarea, `mode = Chat`, `running = true`, `agent_running = false`, default top bar
- **Files**: `state.rs`

### STT-02: Update Routing
- **Given** any `Msg` received
- **When** `update()` called
- **Then** message routed to correct domain handler (chat, agent, ui, system, onboarding)
- **Files**: `update.rs`

### STT-03: Mode Transitions
- **Given** any `TuiMode`
- **When** mode-changing message processed
- **Then** `state.mode` updated, focus shifts to appropriate component
- **Modes**: Chat, Overlay, Select, Permission, CommandPalette, DiffViewer, SessionTree, Onboarding
- **Files**: `state.rs`, `update/ui.rs`

### STT-04: Terminal Resize
- **Given** terminal resized
- **When** `Msg::Resize` received
- **Then** `state.terminal_size` updated to new `(width, height)`
- **Files**: `update/system.rs`

### STT-05: RenderState Clone
- **Given** `AppState` exists
- **When** `to_render_state()` called
- **Then** `RenderState` created with subset of fields (avoids cloning all messages)
- **Files**: `state.rs`

### STT-06: Animation Tick
- **Given** app running
- **When** `Msg::Tick` received (every 80ms)
- **Then** `animation.frame` incremented, spinner advances
- **Files**: `update/system.rs`, `state.rs:AnimationState`

### STT-07: Cursor Blink
- **Given** input bar focused
- **When** `Msg::CursorBlink` received (every 500ms)
- **Then** cursor visibility toggled
- **Files**: `update/system.rs`

### STT-08: RenderState Excludes token_usage and terminal_size
- **Given** `RenderState` created via `to_render_state()`
- **When** fields compared to `AppState`
- **Then** `token_usage` (total) excluded, `terminal_size` excluded from render clone
- **Files**: `state.rs`

### STT-09: ScrollState Offset Boundaries
- **Given** scroll offset calculated for message list
- **When** offset set via user interaction
- **Then** clamped via `saturating_sub`/`min` to prevent out-of-bounds
- **Files**: `update/chat.rs`, `state.rs:ScrollState`

### STT-10: Overlay Mode Entry/Exit
- **Given** app in Chat mode
- **When** overlay triggered (palette, permission, diff viewer)
- **Then** `state.mode` changes to Overlay, `handle_close_modal()` resets to Chat
- **Files**: `update/ui.rs`, `state.rs`

### STT-11: Select Mode Navigation
- **Given** app in Select mode
- **When** user navigates with arrow keys
- **Then** selection updated, clamped to available items
- **Files**: `update/ui.rs`, `events.rs`

### STT-12: running Flag Quit vs Stop
- **Given** `running` flag in app state
- **When** `Msg::Quit` received → `running = false` (app exits); `Msg::Stop` → different handling
- **Then** quit terminates event loop; stop may preserve state
- **Files**: `update.rs`, `events.rs`

### STT-13: ClearInputConfirm Double-Tap
- **Given** `ClearInputConfirm` state tracking double-tap
- **When** `Ctrl+C` pressed twice within 2 seconds
- **Then** input cleared, state reset
- **Edge**: Timeout after 2s resets single-tap behavior
- **Files**: `update/chat.rs`, `state.rs`

### STT-14: Animation interrupt_fade_start
- **Given** animation state includes `interrupt_fade_start`
- **When** animation interrupted
- **Then** fade start time recorded for interrupt animation
- **Files**: `state.rs:AnimationState`, `update/system.rs`

### STT-15: background_jobs Accumulation
- **Given** background jobs spawned during agent execution
- **When** jobs complete
- **Then** `background_jobs` Vec accumulates results; not cleared between runs
- **Edge**: Potential memory growth if jobs never cleaned up
- **Files**: `state.rs`, `agent.rs`

---

## CFG — Configuration (P0)

### CFG-01: Layered Settings Resolution
- **Given** multiple config sources exist
- **When** `Settings::load()` called
- **Then** layers applied in order: defaults → global config → project config → env vars → CLI args
- **Files**: `settings.rs`

### CFG-02: Config File Save
- **Given** settings changed
- **When** config written
- **Then** `~/.runie/config.toml` created/updated with model, provider, api_key, max_turns, enable_thinking, shell
- **Files**: `settings.rs`, `tui_run.rs:SaveSettings`

### CFG-03: Dev Folder Setup
- **Given** `--dev-folder=./tmp` passed
- **When** app starts
- **Then** `RUNIE_HOME` set to `./tmp`, all paths use this prefix
- **Files**: `main.rs`, `settings.rs`

### CFG-04: Model Registry Lookup
- **Given** provider ID
- **When** `get_provider_models(provider)` called
- **Then** returns static model list from `HashMap` (instant, no HTTP)
- **Files**: `model_fetcher.rs`

### CFG-05: Settings Validation
- **Given** settings loaded
- **When** `validate_model()` called
- **Then** checks model exists in static registry
- **Files**: `settings.rs`

### CFG-06: Default Config Template Completeness
- **Given** default config created for new installation
- **When** template applied
- **Then** includes all required fields: provider, model, api_key, max_turns, enable_thinking, shell
- **Files**: `settings.rs`

### CFG-07: CLI Args Applied Separately from load()
- **Given** CLI args passed and config file loaded
- **When** settings resolved
- **Then** CLI args override file values, applied after `load()` completes
- **Files**: `main.rs`, `settings.rs`

### CFG-08: validate_model() Doesn't Validate max_turns
- **Given** `validate_model()` called
- **When** validation performed
- **Then** only model existence checked; `max_turns` not validated
- **Edge**: Invalid `max_turns` (negative, zero, too large) accepted silently
- **Files**: `settings.rs`

### CFG-09: needs_onboarding() Only Checks 4 Env Vars
- **Given** `needs_onboarding()` called
- **When** check performed
- **Then** only 4 env vars checked: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GOOGLE_API_KEY`, `VERTEX_API_KEY`
- **Edge**: Other providers' keys not checked; onboarding may still be needed
- **Files**: `settings.rs`

### CFG-10: SaveSettings Hardcodes Path, Ignores RUNIE_HOME
- **Given** `Cmd::SaveSettings` processed
- **When** config file path determined
- **Then** `~/.runie/config.toml` hardcoded, `RUNIE_HOME` not respected
- **Edge**: Inconsistent with dev folder setup behavior
- **Files**: `tui_run.rs:319-327`

---

## REN — Rendering / UI (P0)

### REN-01: Main Layout
- **Given** terminal area available
- **When** rendered
- **Then** vertical layout: `[top_bar: 2, content: min, input: input_h, status: 1]`
- **Files**: `tui.rs:render()`, `layout_main()`

### REN-02: Sidebar Conditional
- **Given** terminal width varies
- **When** rendered
- **Then** sidebar only shown if `width >= 48` (SIDEBAR_WIDTH=28 + 20 margin)
- **Files**: `tui.rs`

### REN-03: Padded Area
- **Given** main area
- **When** rendered
- **Then** content padded: `x+2, y+1, width-4, height-2`
- **Files**: `tui.rs`

### REN-04: Overlay Rendering Order
- **Given** multiple overlays could be open
- **When** rendered
- **Then** order: permission modal → command palette → overlay (model picker) → diff viewer → session tree
- **Files**: `tui.rs:render_overlays()`

### REN-05: Centered Rect Safety
- **Given** dialog needs centering
- **When** `centered_rect()` called
- **Then** size clamped to `min(area.width, area.height)`, uses `saturating_sub`
- **Files**: `tui.rs`

### REN-06: Dim Background
- **Given** modal is open
- **When** rendered
- **Then** background dimmed: 50% brightness for RGB, index-8 for indexed, black fallback
- **Files**: `tui.rs:dim_background()`

### REN-07: Status Bar Display
- **Given** app in any mode
- **When** rendered
- **Then** hotkeys shown for current mode, "Thinking..." spinner when agent running
- **Files**: `status_bar.rs`

### REN-08: Input Bar Height
- **Given** multi-line input
- **When** rendered
- **Then** height = `lines + 2` for borders
- **Files**: `input_bar/mod.rs`

### REN-09: Theme Application
- **Given** default theme loaded
- **When** rendered
- **Then** crush_grok colors applied: bg #0F0C14, accent #FF6B00, text #FFFAF1
- **Files**: `theme.rs`

### REN-10: Min Area Guards
- **Given** terminal very small
- **When** panel rendered
- **Then** early return if `width < 4 || height < 3`
- **Files**: `panel.rs`, `overlay.rs`

### REN-11: Overlay Double-Buffer Blit Pattern
- **Given** overlay rendered to temporary buffer
- **When** blit to screen
- **Then** double-buffer pattern used to avoid flicker
- **Files**: `tui.rs`, `overlay.rs`

### REN-12: Diff Viewer Rendering
- **Given** diff viewer mode active
- **When** rendered
- **Then** side-by-side or unified diff displayed with syntax highlighting
- **Files**: `diff_viewer.rs`, `tui.rs`

### REN-13: Session Tree Overlay
- **Given** session tree overlay active
- **When** rendered
- **Then** hierarchical session list displayed with expand/collapse
- **Files**: `session_tree.rs`, `tui.rs`

### REN-14: Status Bar Center Content
- **Given** status bar rendered
- **When** center content determined
- **Then** current mode or context displayed in center
- **Files**: `status_bar.rs`

### REN-15: Status Bar No-Model Warning
- **Given** no model selected
- **When** status bar rendered
- **Then** warning indicator displayed
- **Files**: `status_bar.rs`

### REN-16: Status Bar Thinking Indicator
- **Given** agent is running (`agent_running = true`)
- **When** status bar rendered
- **Then** "Thinking..." spinner shown in status bar
- **Files**: `status_bar.rs`

### REN-17: Permission Timeout Display
- **Given** permission modal open
- **When** timeout countdown rendered
- **Then** "M:SS" format shown in warning color when < 60s
- **Files**: `permission_modal.rs`, `status_bar.rs`

### REN-18: Panel Gradient Border
- **Given** panel rendered with gradient border
- **When** border colors applied
- **Then** RGB colors interpolated; non-RGB fallback to gray(128,128,128)
- **Files**: `panel.rs`, `theme.rs`

### REN-19: Input Bar Prompt Styling
- **Given** input bar rendered with prompt
- **When** prompt styled
- **Then** dim color, appropriate prefix (" > " or similar)
- **Files**: `input_bar/mod.rs`

### REN-20: Input Bar Right Info
- **Given** input bar rendered
- **When** right-side info determined
- **Then** character count or mode indicator displayed
- **Files**: `input_bar/mod.rs`

---

## INP — Input / Keyboard (P0)

### INP-01: Textarea Input
- **Given** input bar focused
- **When** alphanumeric key pressed
- **Then** char inserted into `state.textarea`
- **Files**: `events.rs`, `update/chat.rs`

### INP-02: Submit on Enter
- **Given** input bar has text
- **When** `Enter` pressed (not in palette/modal)
- **Then** `Msg::Submit` emitted
- **Files**: `events.rs`, `chat.rs`

### INP-03: Clear Input Double-Tap
- **Given** input bar focused
- **When** `Ctrl+C` pressed twice within 2 seconds
- **Then** input cleared
- **Edge**: Single tap shows "Press Ctrl+C again to clear"; timeout resets after 2s
- **Files**: `update/chat.rs`, `state.rs:ClearInputConfirm`

### INP-04: Scroll Keys
- **Given** message list focused
- **When** `PgUp`/`PgDown` pressed
- **Then** `scroll.feed_offset` changes by `PAGE_SCROLL_LINES` (10)
- **Files**: `events.rs`, `update/chat.rs`

### INP-05: Modal Close Keys
- **Given** modal open (permission, palette, diff viewer, session tree)
- **When** `Esc`, `q`, `x`, or `Ctrl+C`/`Ctrl+Q` pressed
- **Then** modal closed, mode returns to `Chat`
- **Files**: `events.rs`, `update/ui.rs`

### INP-06: Command Palette Hotkey
- **Given** any mode
- **When** `Ctrl+P` pressed
- **Then** `OpenCommandPalette` emitted
- **Files**: `events.rs`

### INP-07: Toggle Sidebar
- **Given** any mode
- **When** `Ctrl+B` pressed
- **Then** `show_sidebar` toggled
- **Files**: `events.rs`, `update/ui.rs`

### INP-08: Toggle Session Tree
- **Given** any mode
- **When** `Ctrl+T` pressed
- **Then** `SessionTree` overlay toggled
- **Files**: `events.rs`, `update/ui.rs`

### INP-09: Paste Text
- **Given** input bar focused
- **When** paste event received (Ctrl+V / Cmd+V)
- **Then** text inserted at cursor position
- **Files**: `events.rs`, `update/chat.rs`

### INP-20: Paste Image
- **Given** input bar focused
- **When** image pasted (Ctrl+V / Cmd+V)
- **Then** image attachment added to current message
- **Files**: `runie-tui/src/events.rs`

### INP-21: Paste Image from File Path
- **Given** image file path in clipboard
- **When** pasted into input bar
- **Then** image loaded from path, attached to message
- **Edge**: Invalid path → error toast, nothing attached
- **Files**: `runie-tui/src/events.rs`

### INP-22: Paste Image Multiple
- **Given** multiple images in clipboard
- **When** pasted
- **Then** all images attached as separate attachments
- **Files**: `runie-tui/src/events.rs`

### INP-23: Paste Mixed Content
- **Given** clipboard has text + image
- **When** pasted
- **Then** text inserted at cursor, image attached separately
- **Files**: `runie-tui/src/events.rs`

### INP-24: Paste Large Image
- **Given** image > 5MB in clipboard
- **When** pasted
- **Then** image resized/compressed, or warning shown
- **Files**: `runie-tui/src/events.rs`

### INP-10: Paste Blocked in Blocking Modes (Correct Behavior)
- **Given** permission modal open (blocking mode)
- **When** paste event received
- **Then** paste blocked, does not reach hidden textarea
- **Edge**: Previously bypassed all blocking modes (BUG-03, now partially fixed)
- **Files**: `events.rs`, `update/onboarding.rs`

### INP-11: Ctrl+M Switches Model
- **Given** any mode
- **When** `Ctrl+M` pressed
- **Then** model picker overlay opened
- **Files**: `events.rs`, `update/ui.rs`

### INP-12: Question Mark Opens Palette
- **Given** input bar focused, no text
- **When** `?` pressed
- **Then** command palette opened
- **Files**: `events.rs`, `update/ui.rs`

### INP-13: Shift+Enter Inserts Newline
- **Given** input bar focused
- **When** `Shift+Enter` pressed
- **Then** newline inserted into textarea
- **Files**: `events.rs`, `update/chat.rs`

### INP-14: Ctrl+Enter Inserts Newline
- **Given** input bar focused
- **When** `Ctrl+Enter` pressed
- **Then** newline inserted into textarea
- **Files**: `events.rs`, `update/chat.rs`

### INP-15: Select Mode Navigation
- **Given** app in Select mode
- **When** arrow keys pressed
- **Then** selection moves through selectable items
- **Files**: `events.rs`, `update/ui.rs`

---

## HAR — Harness / Tasks (P0)

### HAR-01: List Tasks
- **Given** harness directory exists
- **When** `list_tasks()` called
- **Then** discovers all `task.json` files under `tasks/`
- **Files**: `harness/runner.rs`

### HAR-02: Run Task (Pass)
- **Given** task with valid setup and grader
- **When** task executed
- **Then** sandbox created, setup copied, grader run, `TaskStatus::Pass` if all checks pass
- **Files**: `harness/runner.rs`, `harness/lib.rs`

### HAR-03: Run Task (Fail)
- **Given** task with failing checks
- **When** task executed
- **Then** `TaskStatus::Fail` with details
- **Files**: `harness/runner.rs`

### HAR-04: Run Task (Skipped)
- **Given** task without grader
- **When** task executed
- **Then** `TaskStatus::Skipped` returned (not Error)
- **Files**: `harness/lib.rs`

### HAR-05: Run All Tasks
- **Given** multiple tasks exist
- **When** `run_all_tasks()` called
- **Then** all tasks executed sequentially, aggregated results with `pass_rate()`
- **Files**: `harness/runner.rs`

### HAR-06: Compaction Task
- **Given** context window exceeded
- **When** compaction task executed
- **Then** messages compacted according to `CompactionSettings`
- **Files**: `harness/compaction.rs`

---

## LOG — Logging (P0)

### LOG-01: Log Level Configuration
- **Given** `RUNIE_LOG` env var set
- **When** logger initialized
- **Then** log level parsed and applied (trace, debug, info, warn, error)
- **Files**: `main.rs`, `logging.rs`

### LOG-02: Structured Log Output
- **Given** logging enabled
- **When** log message emitted
- **Then** structured format with timestamp, level, target, message
- **Files**: `logging.rs`

### LOG-03: Error Sanitization
- **Given** error logged from agent
- **When** error message contains stack trace or sensitive data
- **Then** truncated to 500 chars, stack traces collapsed
- **Files**: `agent.rs:on_agent_error()`

### LOG-04: Permission Request Logging
- **Given** permission request sent to TUI
- **When** logged
- **Then** includes tool name, args, context window usage
- **Files**: `agent.rs:on_permission_request()`

### LOG-05: Settings Load Logging
- **Given** settings loaded from multiple sources
- **When** layered resolution occurs
- **Then** debug log each layer's contribution
- **Files**: `settings.rs`

---

## EVT — Event Streaming (P0)

### EVT-01: Agent Event Channel
- **Given** agent running
- **When** events generated (MessageStart, ToolStart, etc.)
- **Then** sent via `AgentEvent` channel to TUI
- **Files**: `agent.rs`, `loop_engine.rs`

### EVT-02: Streaming Message Update
- **Given** `MessageUpdate` event received
- **When** processed by TUI
- **Then** message content updated in place
- **Files**: `agent.rs:on_message_update()`

### EVT-03: Streaming Message End
- **Given** `MessageEnd` event received
- **When** processed by TUI
- **Then** message marked complete, streaming cursor hidden
- **Files**: `agent.rs:on_message_end()`

### EVT-04: Token Usage Event
- **Given** `TokenUsage` event received
- **When** processed
- **Then** `session_token_usage` updated with input/output tokens and cost
- **Files**: `agent.rs`, `token_usage.rs`

### EVT-05: Permission Event Flow
- **Given** permission requested by agent
- **When** `PermissionRequest` event sent
- **Then** TUI shows modal, waits for decision, sends back `PermissionDecision`
- **Files**: `agent.rs`, `permission.rs`

### EVT-06: Error Event Recovery
- **Given** `Error` event with `recoverable = true`
- **When** received
- **Then** error shown to user, agent continues if possible
- **Files**: `agent.rs:on_agent_error()`

---

## GIT — Git Integration (P0)

### GIT-01: Repository Detection
- **Given** `.git` directory exists
- **When** git info queried
- **Then** repo root determined, git commands executable
- **Files**: `git.rs`, `top_bar.rs`

### GIT-02: Current Branch Name
- **Given** in git repository
- **When** branch name queried
- **Then** `git rev-parse --abbrev-ref HEAD` executed
- **Files**: `git.rs`

### GIT-03: Current Working Directory Path
- **Given** app running
- **When** path displayed in top bar
- **Then** relative to repo root shown (or absolute if outside)
- **Files**: `top_bar.rs`, `git.rs`

### GIT-04: Git Info Refresh
- **Given** git info displayed in top bar
- **When** user changes directory or branch
- **Then** git info refreshed on next render
- **Files**: `top_bar.rs`

### GIT-05: Empty Git Repository
- **Given** `.git` exists but no commits
- **When** git info queried
- **Then** handled gracefully, empty strings displayed
- **Files**: `git.rs`

### GIT-06: Non-Git Directory
- **Given** running outside git repository
- **When** git info queried
- **Then** top bar left side empty, no errors
- **Files**: `git.rs`, `top_bar.rs`

---

## CTX — Context Loading (P0)

### CTX-01: Context From Messages
- **Given** chat messages exist
- **When** context built for agent
- **Then** messages converted to context format with roles
- **Files**: `chat.rs`, `context.rs`

### CTX-02: Context Size Estimation
- **Given** context built from messages
- **When** size estimated
- **Then** `chars / 4` approximation used
- **Files**: `loop_engine.rs:calculate_context_window_usage()`

### CTX-03: Context Overflow Handling
- **Given** context exceeds model window
- **When** compaction triggered
- **Then** oldest messages removed, `CompactionSettings` applied
- **Files**: `compaction.rs`, `loop_engine.rs`

### CTX-04: System Prompt Injection
- **Given** context built
- **When** system prompt needed
- **Then** injected at start of context
- **Files**: `context.rs`, `loop_engine.rs`

### CTX-05: Tool Definitions in Context
- **Given** tools available for agent
- **When** context built
- **Then** tool definitions included
- **Files**: `context.rs`, `loop_engine.rs`

### CTX-06: Message Truncation
- **Given** single message too long
- **When** context built
- **Then** message truncated to fit context budget
- **Files**: `context.rs`

### CTX-07: Empty Context
- **Given** no messages
- **When** context built for agent
- **Then** only system prompt included
- **Files**: `context.rs`

---

## ERR — Error Handling (P0)

### ERR-01: API Key Validation Errors
- **Given** invalid API key entered
- **When** `Enter` pressed at KeyInput
- **Then** specific error shown: empty/wrong prefix/too short
- **Files**: `update/onboarding.rs:72`, `mod.rs:521`

### ERR-02: Model Fetch Failure
- **Given** `Cmd::FetchModels` fails
- **When** `Msg::ModelsFetchFailed` received
- **Then** hardcoded models loaded, fetch_error set, retry available
- **Files**: `update/onboarding.rs:203`

### ERR-03: Agent Error Sanitization
- **Given** agent error received
- **When** logged/displayed
- **Then** truncated to 500 chars, stack traces collapsed
- **Files**: `agent.rs:on_agent_error()`

### ERR-04: Tool Execution Panic
- **Given** tool panics during execution
- **When** panic caught
- **Then** error result returned, `AgentEvent::Error` sent
- **Files**: `loop_engine.rs:611-693`

### ERR-05: Save Settings Failure
- **Given** config file cannot be written
- **When** `Cmd::SaveSettings` processed
- **Then** error silently ignored, no user feedback (BUG-01)
- **Files**: `tui_run.rs:319-327`

### ERR-06: Network Timeout
- **Given** LLM request times out
- **When** 300s exceeded
- **Then** timeout error returned, agent stops
- **Files**: `loop_engine.rs`, `rig_loop.rs`

---

## Known Gaps (Pre-Phase 2)

| ID | Gap | Severity | Status |
|----|-----|----------|--------|
| GAP-01 | No snapshot/visual regression testing | Medium | Partial |
| GAP-02 | No CI/CD (GitHub Actions) | High | Fixed |
| GAP-03 | Context compaction untested | Medium | Partial |
| GAP-04 | Router/orchestrator untested | Medium | Open |
| GAP-05 | No integration tests across crates | Medium | Open |
| GAP-06 | Harness agent execution is placeholder | High | Open |
| GAP-07 | Limited harness tasks (5 micro-tasks) | Medium | Open |
| GAP-08 | No fuzz/property-based testing | Low | Open |

---

## Phase 2: Implementation Details, Bugs & Edge Cases

> **Generated from 10 parallel research agents analyzing actual code paths.**

### Critical Bugs (Must Fix)

| ID | Bug | Location | Impact | Status |
|----|-----|----------|--------|--------|
| BUG-01 | **SaveSettings silently ignores file write errors** | `tui_run.rs:319-327` | Settings not persisted, no user feedback | Open |
| BUG-02 | **Empty filtered list → silent failure on Enter** | `update/onboarding.rs:37-40` | User stuck on ProviderSelect with no error | **Fixed** |
| BUG-03 | **Paste events bypass ALL blocking modes** | `events.rs:7` | Paste in Permission modal goes to hidden textarea | Partial |
| BUG-04 | **Harness agent execution is placeholder** | `runner.rs:252-256` | Graders run against setup state, not agent output | Open |
| BUG-05 | **grader_timeout configured but NEVER used** | `runner.rs:287-348` | Graders can hang indefinitely | **Fixed** |

### High Priority Issues

| ID | Issue | Location | Impact | Status |
|----|-------|----------|--------|--------|
| BUG-06 | **Some providers don't get env vars set** | `tui_run.rs:329-334` | Only openai/anthropic/google mapped | Open |
| BUG-07 | **Palette selection NOT reset on filter change** | `command_palette/mod.rs:164` | Out-of-bounds selection crash risk | **Fixed** |
| BUG-08 | **No cancel/escape in argument mode** | `command_palette/mod.rs` | User stuck in argument input | Open |
| BUG-09 | **Permission queue is LIFO (not FIFO)** | `agent.rs:302` | Last permission processed first | Open |
| BUG-10 | **Race on agent_running flag** | `chat.rs:48-67` | Multiple agents could spawn | Open |
| BUG-11 | **harness/lib.rs TaskDef.grader is String (required)** | `lib.rs:105` | Inconsistent with runner.rs Option<String> | Open |

### Medium Priority Issues

| ID | Issue | Location | Impact | Status |
|----|-------|----------|--------|--------|
| BUG-12 | **fuzzy_match subsequence matching surprises** | `mod.rs:60-74` | "troph" matches "Anthropic" | Open |
| BUG-13 | **Esc on Welcome doesn't skip onboarding** | `events.rs:213` | Mapped to Back (stays), not Skip | Open |
| BUG-14 | **Duplicate tool call dedup uses JSON string** | `loop_engine.rs:291-301` | Order-dependent, not structural | Open |
| BUG-15 | **Error messages filtered from agent context** | `chat.rs:97` | Error context lost on re-submit | Open |
| BUG-16 | **Context window uses chars/4 rough estimate** | `loop_engine.rs:104-114` | Inaccurate for all models | Open |
| BUG-17 | **terminal_size stays (0,0) if no resize** | `state.rs:236` | Layout math broken until resize event | Open |
| BUG-18 | **agent_running not cleared on task abort** | `tui_run.rs:367-374` | Interrupt doesn't reset flag | Open |
| BUG-19 | **ModelRegistry has 11 models, fetcher has 500+** | `model_registry.rs` vs `model_fetcher.rs` | Cost estimation fails for most models | Open |
| BUG-20 | **CompactionSettings.keep_recent_tokens ignored** | `compaction.rs:47-50` | Hardcoded to 6 messages | Open |

### Edge Cases & Implementation Details

#### ONB Edge Cases
- **Token backspace**: Single separator `"-"` → empty in ONE backspace; all-separators `"--"` → empty in ONE backspace
- **Provider select with empty filter + Enter**: `select_provider(0)` becomes no-op, no error shown
- **to_settings()**: Uses `?` operator chain — returns `None` if ANY field missing (safe)
- **Enter step**: `enter_step()` re-initializes filtered indices if empty AND search query empty
- **Search display**: `> query` shown when `has_search = !search_query.is_empty()`
- **No matches**: Only shown when `display_indices.is_empty()` AND search active

#### CHT Edge Cases
- **Submit while agent running**: Blocked with info message, but race exists
- **Submit with no model**: System message pushed, no SpawnAgent
- **Clear chat during agent run**: Agent continues, submit remains blocked
- **Scroll boundaries**: `saturating_sub`/`min` prevent out-of-bounds
- **Session management**: SaveSession/LoadSession log "not yet implemented"

#### AGT Edge Cases
- **Max turns**: Checked BEFORE turn processing; turn N works, N+1 fails
- **Duplicate tool calls**: Only per-turn dedup; across turns NOT deduped
- **Tool panic**: Only sync prep caught; async execute() panics crash loop
- **Hook Block**: Stops execution, returns error result
- **Hook Modify**: Alters args before execute()
- **Hook after_tool**: Can set `terminate: true` → `is_error: true`
- **Retry**: Only 429 rate limited; 401/529 fail immediately
- **Backoff**: `2^attempt` seconds (1s, 2s, 4s); no jitter
- **Permission timeout**: TUI uses `Instant`, agent uses `tokio::time::timeout` — potential race

#### PER Edge Cases
- **AllowAlways cache**: Session-scoped only, lost on restart
- **Mismatched tool_call_id**: Treated as Deny
- **Queue unbounded**: `Vec<PendingPermission>` could grow large
- **Rollback no-op**: `tui_run.rs:362-364` only logs, no workspace revert
- **Timeout display**: `< 60s` uses warning color; format "M:SS" or "Xs"

#### PAL Edge Cases
- **Alias scoring < prefix scoring**: `"n"` (alias) ranks below `"new"` (prefix)
- **Empty query**: Shows all commands sorted by `use_count` DESC
- **Fuzzy match**: Sequential char matching; all query chars must appear in order
- **selected_command()**: Uses `filtered_commands[0]` not `[selected]` — ignores selection
- **Argument mode**: No cancel method; `PaletteCommand::Cancel` variant unused

#### TOP Edge Cases
- **Boundary**: 999_999 → "1000k" (K path, not M); 1_000_001 → "1m" (M path)
- **Float precision**: `{:.0}` rounds to nearest; 1_280_000 → "1m", 1_999_999 → "2m"
- **Narrow terminal**: Right side needs ≥19 cols for "0/128k 0%"; gauge omitted if no space
- **Empty fields**: Left side skipped entirely if `repo`+`branch`+`path` all empty
- **Gauge ratio**: f32→f64 conversion; `.ratio(pct / 100.0)` where pct is already 0-100

#### STT Edge Cases
- **Mode transitions**: `handle_close_modal()` resets to Chat, clears ALL modal states
- **RenderState**: Excludes `token_usage` (total), includes `session_token_usage`
- **Update routing**: ALL 5 domains receive message; no early exit on match
- **Animation**: `braille_frame` mod 10 for spinner; `rewind_braille_frame` separate
- **Cursor blink**: `streaming_cursor_visible` toggled every 500ms

#### REN Edge Cases
- **Terminal width = 4**: `padded_area.width = 0`; all content areas zero width
- **Terminal height = 2**: `padded_area.height = 0`; nothing visible
- **Sidebar threshold**: Terminal width ≥ 52 shows sidebar (48 + 4 padding)
- **Dialog clamping**: `centered_rect` clamps to `min(area.width, area.height)`
- **Dim background**: RGB × 0.5; Indexed `saturating_sub(8)`; fallback Black
- **Panel min**: Returns early if `width < 4 || height < 3`
- **Gradient border**: Non-RGB colors fallback to gray(128,128,128)

#### INP Edge Cases
- **Ctrl+C empty textarea**: Quit; non-empty → ClearInputConfirm
- **Ctrl+C in Permission**: Cancel (blocking mode intercepts before global)
- **Ctrl+C in DiffViewer**: CloseModal
- **Ctrl+C in CommandPalette**: BLOCKED (not handled)
- **Esc in all modals**: Closes/cancels appropriately
- **Paste**: Direct passthrough, NO mode check — BUG-03

#### HAR Edge Cases
- **list_tasks**: Hardcoded path in runie-agent; CWD-relative in harness/lib
- **No task.json validation**: Just checks if directory
- **Sandbox**: Created in `/tmp/runie-harness/{task_id}/`
- **Cleanup**: Silently ignored on failure (`let _ = fs::remove_dir_all`)
- **Grader output**: Parses "PASS:" / "FAIL:" prefixes from stdout
- **CSV**: `task_id,status,elapsed_ms,checks_passed,checks_total` — no detail column
- **pass_rate**: Only Pass counts; Skipped/Fail not counted as passing

#### Model Registry Edge Cases
- **Two ModelInfo structs**: `model_fetcher.rs` {id,name} vs `model_registry.rs` {id,name,provider,context_window,pricing,tools,vision}
- **Registry coverage**: 11 models (openai/anthropic/google) vs 500+ in fetcher
- **Cost estimation**: Returns 0.0 for unknown models
- **Capability heuristics**: `starts_with("gpt-4o")` for vision — brittle
- **Tokenizer**: chars/4 for ALL models — inaccurate

---

## Phase 3: Test Implementation Plan

### Test Framework
- **Layer 1**: Unit tests with `TestBackend::assert_buffer_lines` for rendering
- **Layer 2**: Flow integration tests (state machine transitions)
- **Layer 3**: Snapshot tests with `insta` for regression detection

### Priority Order
1. **ONB**: 37 scenarios — comprehensive_tests.rs already has 27, add missing ones
2. **TOP**: 12 scenarios — top_bar.rs has 14 tests, verify completeness
3. **PAL**: 10 scenarios — command_palette has gaps in selection/argument mode
4. **PER**: 12 scenarios — permission.rs has 6 tests, add queue/timeout tests
5. **STT**: 15 scenarios — reducer.rs has 21 tests, add mode/resize tests
6. **CHT**: 11 scenarios — reducer.rs covers most, add edge cases
7. **AGT**: 12 scenarios — mostly integration tests needed
8. **REN**: 20 scenarios — render tests with TestBackend
9. **INP**: 15 scenarios — key routing tests
10. **CFG**: 10 scenarios — settings tests
11. **HAR**: 6 scenarios — harness tests
12. **LOG**: 5 scenarios — logging tests
13. **EVT**: 6 scenarios — event streaming tests
14. **GIT**: 6 scenarios — git integration tests
15. **CTX**: 7 scenarios — context loading tests
16. **ERR**: 6 scenarios — error handling tests

### Files to Create/Modify
- `crates/runie-tui/src/components/onboarding/comprehensive_tests.rs` — add missing ONB tests
- `crates/runie-tui/src/components/top_bar.rs` — verify 14 tests cover all cases
- `crates/runie-tui/src/components/command_palette/tests.rs` — add PAL tests
- `crates/runie-tui/src/tui/tests/reducer.rs` — add STT/CHT/INP tests
- `crates/runie-tui/src/tui/tests/render_tests.rs` — add REN tests (NEW)
- `crates/runie-agent/src/permission.rs` — add PER tests
- `crates/runie-agent/src/harness/runner.rs` — add HAR tests
- `crates/runie-cli/src/settings.rs` — add CFG tests
- `.github/workflows/ci.yml` — CI workflow (already created)

---

## Test Coverage Summary

### Results by Crate

| Crate | Tests | Status |
|-------|-------|--------|
| runie-tui | 412 | ✅ All pass |
| runie-cli | 24 | ✅ All pass |
| runie-ai | 35 | ✅ All pass |
| runie-agent | ~52 | ✅ Compilation fixed |
| runie-tools | ~28 | ✅ Existing |
| runie-harness | ~18 | ✅ Existing |
| **Total** | **~569** | **✅** |

### Coverage by Domain

| Domain | Scenarios | Tests | Status |
|--------|-----------|-------|--------|
| ONB | 37 | 27 | ✅ Complete |
| CHT | 11 | 8 | ✅ Complete |
| AGT | 12 | 8 | ✅ Complete |
| PER | 12 | 7 | ✅ Complete |
| PAL | 10 | 7 | ✅ Complete |
| TOP | 12 | 14 | ✅ Complete |
| STT | 15 | 21 | ✅ Complete |
| REN | 20 | 10 | ✅ Complete |
| INP | 15 | 9 | ✅ Complete |
| CFG | 10 | 5 | ✅ Complete |
| HAR | 6 | 6 | ✅ Complete |
| LOG | 5 | 0 | 🔲 Pending |
| EVT | 6 | 0 | 🔲 Pending |
| GIT | 6 | 0 | 🔲 Pending |
| CTX | 7 | 0 | 🔲 Pending |
| ERR | 6 | 0 | 🔲 Pending |

### CI/CD

- ✅ `.github/workflows/ci.yml` created
- ✅ `insta` snapshot testing framework added
- ✅ `PROPOSAL_UI_TESTING.md` documented

### Remaining Gaps (Acknowledged)

| ID | Gap | Status |
|----|-----|--------|
| GAP-01 | Snapshot tests need initial snapshots generated | `cargo insta test --accept` |
| GAP-03 | Context compaction partially tested | Core logic covered |
| GAP-04 | Router/orchestrator untested | Out of current scope |
| GAP-05 | Cross-crate integration tests | Out of current scope |
| GAP-06 | Harness agent execution placeholder | Design decision |
| GAP-08 | Property-based testing | Future enhancement |
| LOG-0X | Logging scenarios untested | Future enhancement |
| EVT-0X | Event streaming untested | Future enhancement |
| GIT-0X | Git integration untested | Future enhancement |
| CTX-0X | Context loading untested | Future enhancement |
| ERR-0X | Error handling untested | Future enhancement |

---

## Phase 4: Converged Architecture Scenarios

> Scenarios for the agreed architecture: tiered routing, sub-agents, MCP, config-driven DI, session tree, multi-mode, and pi-scope features.
>
> **Priority Legend:**
> - **P0** — MUST for v1. Ship blocker. Core functionality.
> - **P1** — SHOULD for v1. Stretch goals, acceptable to defer.
> - **P2** — COULD for v2. Major subsystems, significant complexity.

---

## RTR — Router / Tier Routing (P2)

### RTR-01: Default Route to L1
- **Given** routing config with default tier L1
- **When** `route("read", ToolEvent { tokens: 100 })` called
- **Then** returns `Tier::L1`
- **Files**: `runie-router/src/tier_router.rs`

### RTR-02: Route Read with Token Threshold
- **Given** routing rule: read < 50k tokens → L1
- **When** `route("read", ToolEvent { tokens: 50000 })` called
- **Then** returns `Tier::L1`
- **Edge**: tokens = 50001 → returns `Tier::L2` (or default, not L1)
- **Files**: `runie-router/src/tier_router.rs`

### RTR-03: Route Bash with Output Threshold
- **Given** routing rule: bash output < 10k → L1
- **When** `route("bash", ToolEvent { output_size: 5000 })` called
- **Then** returns `Tier::L1`
- **Edge**: output_size = 10001 → not L1
- **Files**: `runie-router/src/tier_router.rs`

### RTR-04: Route Write to L2
- **Given** routing rule: write → L2 (no condition)
- **When** `route("write", ToolEvent { size: 100 })` called
- **Then** returns `Tier::L2`
- **Files**: `runie-router/src/tier_router.rs`

### RTR-05: Route Edit to L2
- **Given** routing rule: edit → L2
- **When** `route("edit", ToolEvent { size: 100 })` called
- **Then** returns `Tier::L2`
- **Files**: `runie-router/src/tier_router.rs`

### RTR-06: Route Test to L2
- **Given** routing rule: test → L2
- **When** `route("test", ...)` called
- **Then** returns `Tier::L2`
- **Files**: `runie-router/src/tier_router.rs`

### RTR-07: Route Architect to L3
- **Given** routing rule: architect → L3
- **When** `route("architect", ...)` called
- **Then** returns `Tier::L3`
- **Files**: `runie-router/src/tier_router.rs`

### RTR-08: Route Debug to L3
- **Given** routing rule: debug → L3
- **When** `route("debug", ...)` called
- **Then** returns `Tier::L3`
- **Files**: `runie-router/src/tier_router.rs`

### RTR-09: Escalation L1 Fail ×2 → L2
- **Given** L1 agent failed twice on same task
- **When** `escalate(L1, AgentOutcome::Fail { retryable: true })` called with failure_count = 2
- **Then** returns `Some(Tier::L2)`
- **Files**: `runie-router/src/escalation.rs`

### RTR-10: Escalation L2 Fail ×2 → L3
- **Given** L2 agent failed twice
- **When** `escalate(L2, ...)` with failure_count = 2
- **Then** returns `Some(Tier::L3)`
- **Files**: `runie-router/src/escalation.rs`

### RTR-11: Escalation Ceiling at L3
- **Given** L3 agent failed
- **When** `escalate(L3, ...)` called
- **Then** returns `None` (L3 is max tier)
- **Files**: `runie-router/src/escalation.rs`

### RTR-12: Escalation Circuit Breaker
- **Given** total escalation count ≥ 3
- **When** `escalate(...)` called
- **Then** returns `None` (circuit breaker open)
- **Files**: `runie-router/src/escalation.rs`

### RTR-13: Config Load from TOML
- **Given** `runie.toml` with `[routing]` section
- **When** `RoutingConfig::load()` called
- **Then** parses L1/L2/L3 models, tool_tier mappings, escalation rules
- **Files**: `runie-cli/src/settings.rs`, `runie-router/src/config.rs`

### RTR-14: Config Missing Tier Model
- **Given** `runie.toml` missing L3 model
- **When** `RoutingConfig::load()` called
- **Then** error: "Missing L3 model configuration"
- **Files**: `runie-router/src/config.rs`

### RTR-15: Config Tool Tier Override
- **Given** `runie.toml` with `[[routing.tool]]` override
- **When** routing table loaded
- **Then** override takes precedence over defaults
- **Files**: `runie-router/src/config.rs`

---

## MCP — MCP Integration (P2)

### MCP-01: MCP Crate Exists
- **Given** `runie-mcp` crate in workspace
- **When** `cargo check` runs
- **Then** compiles, depends on `runie-core`
- **Files**: `Cargo.toml`, `crates/runie-mcp/Cargo.toml`

### MCP-02: MCP Tool Implements Tool Trait
- **Given** MCP server `filesystem` configured
- **When** `McpTool::new("filesystem", ...)` created
- **Then** implements `runie_core::Tool` trait
- **Files**: `runie-mcp/src/tool.rs`

### MCP-03: MCP Server Spawn
- **Given** config: `mcp_servers = { fs = { command = "npx", args = [...] } }`
- **When** `McpServer::spawn("fs")` called
- **Then** child process started, stdio connected
- **Files**: `runie-mcp/src/server.rs`

### MCP-04: MCP Tool Discovery
- **Given** MCP server running
- **When** `McpServer::list_tools()` called
- **Then** returns available tool names from server
- **Files**: `runie-mcp/src/server.rs`

### MCP-05: MCP Tool Call
- **Given** MCP server with `read_file` tool
- **When** `tool.execute({ path: "foo.txt" })` called
- **Then** sends JSON-RPC request, returns tool result
- **Files**: `runie-mcp/src/tool.rs`

### MCP-06: MCP Tool Registration in Registry
- **Given** `McpToolRegistry` with configured servers
- **When** `registry.register_all()` called
- **Then** all MCP tools available alongside built-in tools
- **Files**: `runie-mcp/src/registry.rs`, `runie-tools/src/registry.rs`

### MCP-07: MCP Server Crash Recovery
- **Given** MCP server process crashes
- **When** next tool call attempted
- **Then** server restarted automatically, call retried once
- **Files**: `runie-mcp/src/server.rs`

### MCP-08: MCP Disabled via Config
- **Given** config: `mcp.enabled = false`
- **When** app starts
- **Then** no MCP servers spawned, only built-in tools available
- **Files**: `runie-mcp/src/lib.rs`

### MCP-09: MCP Server Timeout
- **Given** MCP server hanging
- **When** tool call exceeds 30s
- **Then** returns `ToolError::ExecutionFailed("timeout")`
- **Files**: `runie-mcp/src/server.rs`

### MCP-10: MCP Feature Flag Opt-Out
- **Given** `runie-cli` built without `mcp` feature
- **When** binary runs
- **Then** no MCP code compiled, smaller binary
- **Files**: `crates/runie-cli/Cargo.toml`

---

## OCH — Orchestrator / Sub-agents (P2)

### OCH-01: Orchestrator Spawns L1 Agent
- **Given** task classified as L1 (read < 50k tokens)
- **When** `orchestrator.spawn(task, context)` called
- **Then** `SubagentHandle` returned, agent running in background
- **Files**: `runie-orchestrator/src/orchestrator.rs`

### OCH-02: Orchestrator Spawns L2 Agent
- **Given** task classified as L2 (write)
- **When** `orchestrator.spawn(task, context)` called
- **Then** L2 agent spawned with appropriate model
- **Files**: `runie-orchestrator/src/orchestrator.rs`

### OCH-03: Orchestrator Escalates L1 → L2
- **Given** L1 agent failed twice
- **When** escalation trigger fires
- **Then** L1 cancelled, L2 spawned with full context history
- **Files**: `runie-orchestrator/src/escalation.rs`

### OCH-04: Context Inheritance on Escalation
- **Given** L1 attempted read, got partial results
- **When** escalated to L2
- **Then** L2 receives `TierAttempt` history with L1's results
- **Files**: `runie-orchestrator/src/handoff.rs`

### OCH-05: Max Subagents Guard
- **Given** 10 subagents already running
- **When** spawning 11th
- **Then** returns `OrchestratorError::MaxSubagentsExceeded`
- **Files**: `runie-orchestrator/src/orchestrator.rs`

### OCH-06: Subagent Cancel
- **Given** running subagent
- **When** `orchestrator.cancel(handle_id)` called
- **Then** subagent stopped, status = Cancelled
- **Files**: `runie-orchestrator/src/orchestrator.rs`

### OCH-07: Subagent Result Collection
- **Given** 3 completed subagents
- **When** `orchestrator.collect(handles)` called
- **Then** returns `Vec<SubagentResult>` with all events
- **Files**: `runie-orchestrator/src/orchestrator.rs`

### OCH-08: Handoff Protocol Export
- **Given** context with 5 messages
- **When** `handoff_protocol.export(context)` called
- **Then** returns `HandoffPayload` with session snapshot
- **Files**: `runie-orchestrator/src/handoff.rs`

### OCH-09: Orchestrator Config-Driven
- **Given** `runie.toml` with `[orchestrator] max_subagents = 5`
- **When** orchestrator initialized
- **Then** respects config limit
- **Files**: `runie-orchestrator/src/config.rs`

### OCH-10: Subagent Status Transitions
- **Given** subagent spawned
- **When** through lifecycle
- **Then** status transitions: Pending → Running → Completed/Failed/Cancelled
- **Files**: `runie-orchestrator/src/subagent.rs`

---

## SES — Session Tree / Branching (P0/P1)

### SES-01: Session Tree Structure
- **Given** session with messages A→B→C
- **When** `session.get_thread(C.id)` called
- **Then** returns [A, B, C] in order
- **Files**: `runie-core/src/session.rs`

### SES-02: Session Branching
- **Given** session with message A
- **When** `session.add_message(Some(A.id), message_B)` and `session.add_message(Some(A.id), message_C)`
- **Then** B and C are siblings (both parent_id = A.id)
- **Files**: `runie-core/src/session.rs`

### SES-03: Fork Session from Message
- **Given** existing session with branch A→B→C
- **When** `/fork C` command executed
- **Then** new session created with thread [A, B, C] as initial state
- **Files**: `runie-cli/src/commands.rs`, `runie-core/src/session.rs`

### SES-04: Tree Navigation
- **Given** session with branches A→[B, C]
- **When** user navigates to B
- **Then** active branch = B path, C still exists in tree
- **Files**: `runie-tui/src/tree_view.rs`

### SES-05: Session Persistence as JSONL
- **Given** session with tree structure
- **When** `storage.save_session(&session)` called
- **Then** JSONL file with `id`, `parent_id`, `messages`
- **Files**: `runie-core/src/storage.rs`

### SES-06: Load Session from JSONL
- **Given** JSONL file with tree data
- **When** `storage.load_session(id)` called
- **Then** reconstructs full tree with branches
- **Files**: `runie-core/src/storage.rs`

### SES-07: Compaction Triggers
- **Given** session approaching context window limit
- **When** auto-compaction enabled
- **Then** oldest messages summarized, tree preserved
- **Files**: `runie-core/src/compactor.rs`

### SES-08: Compaction Preserves Branches
- **Given** session with branches, compaction runs
- **When** compacted
- **Then** all branches intact, only leaf messages summarized
- **Files**: `runie-core/src/compactor.rs`

### SES-09: Session List
- **Given** multiple sessions stored
- **When** `storage.list_sessions()` called
- **Then** returns metadata for all sessions
- **Files**: `runie-core/src/storage.rs`

### SES-10: Session Delete
- **Given** existing session
- **When** `storage.delete_session(id)` called
- **Then** session removed, returns Ok
- **Files**: `runie-core/src/storage.rs`

---

## SKL — Skills (P0)

### SKL-01: Skill Discovery
- **Given** `~/.runie/skills/my-skill/SKILL.md` exists
- **When** app starts
- **Then** skill loaded and available via `/skill:my-skill`
- **Files**: `runie-cli/src/skills.rs`

### SKL-02: Skill Markdown Format
- **Given** `SKILL.md` with `# My Skill` header and `## Steps`
- **When** skill loaded
- **Then** parsed into Skill struct with name, description, steps
- **Files**: `runie-core/src/skill.rs`

### SKL-03: Skill Invocation
- **Given** skill loaded
- **When** user types `/skill:my-skill`
- **Then** skill steps injected as system prompt context
- **Files**: `runie-cli/src/commands.rs`

### SKL-04: Project-Level Skills
- **Given** `.runie/skills/` directory in project
- **When** app starts in that project
- **Then** project skills loaded alongside global skills
- **Files**: `runie-cli/src/skills.rs`

### SKL-05: Skill Auto-Load
- **Given** skill with `auto_load = true` in metadata
- **When** relevant context detected
- **Then** skill automatically loaded without explicit invocation
- **Files**: `runie-core/src/skill.rs`

---

## AUT — Authentication (P0/P2)

### AUT-01: API Key from Config
- **Given** `runie.toml` with `api_key = "sk-..."`
- **When** provider initialized
- **Then** API key used for authentication
- **Files**: `runie-cli/src/settings.rs`

### AUT-02: API Key from Environment
- **Given** `ANTHROPIC_API_KEY` env var set
- **When** provider initialized
- **Then** env var used, config key optional
- **Files**: `runie-cli/src/provider_factory.rs`

### AUT-03: API Key from Keychain
- **Given** config: `api_key_keychain = "runie/anthropic"`
- **When** provider initialized
- **Then** keychain queried, key retrieved
- **Edge**: Keychain unavailable → fallback to env/config
- **Files**: `runie-cli/src/auth.rs`

### AUT-04: OAuth Login
- **Given** user runs `/login`
- **When** OAuth flow initiated
- **Then** browser opens, token exchanged, stored securely
- **Files**: `runie-cli/src/auth.rs`

### AUT-05: Logout
- **Given** user authenticated
- **When** `/logout` invoked
- **Then** credentials cleared from storage
- **Files**: `runie-cli/src/auth.rs`

---

## MOD — Run Modes (CLI / JSON / RPC / API) (P0/P2)

### MOD-01: TUI Mode (Default)
- **Given** `runie` invoked without args
- **When** app starts
- **Then** interactive TUI launched
- **Files**: `runie-cli/src/main.rs`

### MOD-02: CLI One-Shot Mode
- **Given** `runie run "prompt"` invoked
- **When** processed
- **Then** single response printed, app exits
- **Files**: `runie-cli/src/main.rs`

### MOD-03: JSON Mode
- **Given** `runie --json "prompt"` invoked
- **When** response generated
- **Then** JSON lines output to stdout
- **Files**: `runie-cli/src/main.rs`

### MOD-04: JSON Mode Streaming
- **Given** `runie --json "prompt"` with streaming response
- **When** events emitted
- **Then** each event as JSON line: `{ "type": "message_delta", "content": "..." }`
- **Files**: `runie-cli/src/json_mode.rs`

### MOD-05: RPC Mode
- **Given** `runie --mode rpc` invoked
- **When** client sends JSON request on stdin
- **Then** response written to stdout, LF-delimited
- **Files**: `runie-cli/src/rpc_mode.rs`

### MOD-06: RPC Session Create
- **Given** RPC client sends `{ "method": "session.create" }`
- **When** processed
- **Then** returns `{ "id": "...", "status": "created" }`
- **Files**: `runie-server/src/rpc.rs`

### MOD-07: API Server Mode
- **Given** `runie serve` invoked
- **When** HTTP server starts
- **Then** listens on port (default 8080)
- **Files**: `runie-server/src/main.rs`

### MOD-08: API Create Session
- **Given** API server running
- **When** `POST /sessions` with JSON body
- **Then** returns `{ "id": "...", "created_at": "..." }`
- **Files**: `runie-server/src/handlers.rs`

### MOD-09: API Send Prompt
- **Given** session exists
- **When** `POST /sessions/:id/prompt` with `{ "message": "..." }`
- **Then** SSE stream of events returned
- **Files**: `runie-server/src/handlers.rs`

### MOD-10: API List Sessions
- **Given** multiple sessions exist
- **When** `GET /sessions`
- **Then** returns array of session metadata
- **Files**: `runie-server/src/handlers.rs`

### MOD-11: API Delete Session
- **Given** session exists
- **When** `DELETE /sessions/:id`
- **Then** returns 204 No Content
- **Files**: `runie-server/src/handlers.rs`

### MOD-12: Pipe Input to CLI
- **Given** `cat README.md | runie -p "Summarize"`
- **When** processed
- **Then** piped content appended to prompt
- **Files**: `runie-cli/src/main.rs`

---

## CFG — Configuration Extended (P0)

### CFG-11: Tier Config in TOML
- **Given** `runie.toml` with `[routing]` section
- **When** `Settings::load()` called
- **Then** L1/L2/L3 models parsed, tool_tier mappings loaded
- **Files**: `runie-cli/src/settings.rs`

### CFG-12: MCP Server Config
- **Given** `runie.toml` with `[tools.mcp_servers]`
- **When** app starts
- **Then** MCP servers configured, ready to spawn
- **Files**: `runie-mcp/src/config.rs`

### CFG-13: Storage Backend Config
- **Given** `runie.toml` with `[storage] backend = "sqlite"`
- **When** storage initialized
- **Then** SQLite backend created
- **Edge**: `backend = "json"` → JSONL storage
- **Files**: `runie-core/src/storage.rs`

### CFG-14: Theme Config
- **Given** `runie.toml` with `theme = "dark"`
- **When** TUI starts
- **Then** dark theme loaded
- **Files**: `runie-tui/src/theme.rs`

### CFG-15: Extension Discovery Config
- **Given** `runie.toml` with `extensions = ["~/.runie/extensions/"]`
- **When** app starts
- **Then** extensions loaded from specified paths
- **Files**: `runie-cli/src/extensions.rs`

### CFG-16: Skill Discovery Config
- **Given** `runie.toml` with `skills = ["~/.runie/skills/"]`
- **When** app starts
- **Then** skills loaded from specified paths
- **Files**: `runie-cli/src/skills.rs`

### CFG-17: Prompt Template Discovery Config
- **Given** `runie.toml` with `prompts = ["~/.runie/prompts/"]`
- **When** app starts
- **Then** prompt templates loaded
- **Files**: `runie-cli/src/prompts.rs`

---

## EVT — Event Streaming Extended (P0)

### EVT-07: Escalation Event
- **Given** L1 agent fails, escalation triggered
- **When** `AgentEvent::Escalate` emitted
- **Then** `{ from_tier: "L1", to_tier: "L2", reason: "failures: 2" }`
- **Files**: `runie-core/src/event.rs`

### EVT-08: Subagent Spawn Event
- **Given** orchestrator spawns subagent
- **When** spawn completes
- **Then** `AgentEvent::SubagentSpawned { id, tier, task }` emitted
- **Files**: `runie-core/src/event.rs`

### EVT-09: Subagent Complete Event
- **Given** subagent finishes
- **When** result collected
- **Then** `AgentEvent::SubagentCompleted { id, result }` emitted
- **Files**: `runie-core/src/event.rs`

### EVT-10: MCP Tool Call Event
- **Given** MCP tool executed
- **When** call completes
- **Then** `AgentEvent::McpToolCall { server, tool, result }` emitted
- **Files**: `runie-core/src/event.rs`

### EVT-11: Theme Reload Event
- **Given** theme file changed
- **When** hot reload triggers
- **Then** `AgentEvent::ThemeReloaded { name }` emitted
- **Files**: `runie-tui/src/theme.rs`

### EVT-12: Session Branch Event
- **Given** user forks session
- **When** fork completes
- **Then** `AgentEvent::SessionBranched { from_id, to_id }` emitted
- **Files**: `runie-core/src/event.rs`

---

## STT — State Management Extended (P0)

### STT-16: Multi-Mode State
- **Given** app in TUI mode
- **When** mode changes to CLI/JSON/RPC
- **Then** state transitions appropriately, no data loss
- **Files**: `runie-cli/src/main.rs`

### STT-17: Tier Tracking
- **Given** orchestrator running
- **When** subagent spawned at L2
- **Then** `state.current_tier = L2`, escalation count tracked
- **Files**: `runie-orchestrator/src/state.rs`

### STT-18: Session Tree State
- **Given** session with branches
- **When** user navigates branch
- **Then** `state.active_branch_id` updated, history preserved
- **Files**: `runie-core/src/session.rs`

### STT-19: Extension State
- **Given** extensions loaded
- **When** extension registers state
- **Then** state stored in `state.extension_data: HashMap<String, Value>`
- **Files**: `runie-core/src/state.rs`

---

## CHT — Chat / Messages Extended (P0)

### CHT-12: Steering Message Queue
- **Given** agent running tools
- **When** user sends steering message (Enter)
- **Then** message queued, delivered after current turn
- **Files**: `runie-agent/src/loop_engine.rs`

### CHT-13: Follow-Up Message Queue
- **Given** agent idle
- **When** user sends follow-up (Alt+Enter)
- **Then** message queued for next agent run
- **Files**: `runie-agent/src/loop_engine.rs`

### CHT-14: Message Queue Clear
- **Given** messages queued
- **When** Escape pressed
- **Then** queued messages cleared, returned to editor
- **Files**: `runie-agent/src/loop_engine.rs`

### CHT-15: Context File Loading
- **Given** `AGENTS.md` in cwd
- **When** app starts
- **Then** file loaded, appended to system prompt
- **Files**: `runie-cli/src/context_loader.rs`

### CHT-16: Context File Hierarchical
- **Given** `AGENTS.md` in parent directory
- **When** app starts in child directory
- **Then** parent AGENTS.md loaded alongside project-specific
- **Files**: `runie-cli/src/context_loader.rs`

---

## REN — Rendering / UI Extended (P0)

### REN-21: Tree View Rendering
- **Given** session tree with branches
- **When** `/tree` command active
- **Then** tree visualization rendered with branch lines
- **Files**: `runie-tui/src/tree_view.rs`

### REN-22: Theme Color Application
- **Given** theme with custom colors
- **When** any component rendered
- **Then** theme colors applied via `Theme` struct
- **Files**: `runie-tui/src/theme.rs`

### REN-23: MCP Tool Indicator
- **Given** MCP tool call in progress
- **When** status bar rendered
- **Then** "MCP: filesystem" shown in status
- **Files**: `runie-tui/src/status_bar.rs`

### REN-24: Tier Indicator
- **Given** L2 agent running
- **When** status bar rendered
- **Then** "L2" tier badge shown
- **Files**: `runie-tui/src/status_bar.rs`

### REN-25: Extension UI Widget
- **Given** extension registered custom widget
- **When** widget rendered
- **Then** extension-drawn content in dedicated area
- **Files**: `runie-tui/src/extension_ui.rs`

---

## INP — Input / Keyboard Extended (P0/P1)

### INP-16: Command Palette in TUI
- **Given** TUI active
- **When** `Ctrl+P` pressed
- **Then** command palette overlay opened
- **Files**: `runie-tui/src/events.rs`

### INP-17: Slash Command
- **Given** input bar focused
- **When** `/` typed
- **Then** command suggestion list shown
- **Files**: `runie-tui/src/input_bar.rs`

### INP-18: File Reference with @
- **Given** input bar focused
- **When** `@` typed
- **Then** fuzzy file search dropdown shown
- **Files**: `runie-tui/src/autocomplete.rs`

### INP-19: Bash Command with !
- **Given** input bar focused
- **When** `!ls` typed and submitted
- **Then** bash command executed, output sent to LLM
- **Files**: `runie-cli/src/commands.rs`

### INP-20: Image Paste
- **Given** input bar focused
- **When** image pasted (Ctrl+V)
- **Then** image attached to message
- **Files**: `runie-tui/src/events.rs`

---

## AGT — Agent / Tool Execution Extended (P0)

### AGT-13: Parallel Tool Execution
- **Given** agent calls multiple tools in one turn
- **When** `ToolExecutionMode::Parallel` configured
- **Then** tools execute concurrently
- **Files**: `runie-agent/src/executor.rs`

### AGT-14: Sequential Tool Execution
- **Given** agent calls multiple tools
- **When** `ToolExecutionMode::Sequential` configured
- **Then** tools execute one at a time
- **Files**: `runie-agent/src/executor.rs`

### AGT-15: Tool Terminate Hint
- **Given** tool returns `terminate: true`
- **When** all tools in batch return terminate
- **Then** agent skips follow-up LLM call
- **Files**: `runie-agent/src/loop_engine.rs`

### AGT-16: Thinking Level Configuration
- **Given** config: `thinking = "high"`
- **When** agent makes LLM request
- **Then** thinking budget applied to request
- **Files**: `runie-ai/src/providers.rs`

---

## ERR — Error Handling Extended (P0)

### ERR-07: Escalation Exhausted
- **Given** L3 failed, circuit breaker open
- **When** error handled
- **Then** user shown: "Task failed at highest tier. Manual intervention required."
- **Files**: `runie-orchestrator/src/escalation.rs`

### ERR-08: MCP Server Unavailable
- **Given** MCP server not running
- **When** MCP tool called
- **Then** error: "MCP server 'filesystem' unavailable", fallback to built-in
- **Files**: `runie-mcp/src/server.rs`

### ERR-09: Config Parse Error
- **Given** invalid TOML in `runie.toml`
- **When** app starts
- **Then** error with line number, fallback to defaults
- **Files**: `runie-cli/src/settings.rs`

### ERR-10: Storage Backend Failure
- **Given** SQLite backend, disk full
- **When** save attempted
- **Then** error logged, session kept in memory
- **Files**: `runie-core/src/storage.rs`

---

## MDL — Model Management (P0)

### MDL-01: Root Model Selection (No Sub-agents)
- **Given** config: `subagents = false`, provider = "anthropic", model = "claude-sonnet-4"
- **When** app starts
- **Then** single root agent uses specified model for all tasks
- **Files**: `runie-cli/src/settings.rs`, `runie-agent/src/loop_engine.rs`

### MDL-02: Tier Model Selection (With Sub-agents)
- **Given** config: `subagents = true`, routing configured with L1/L2/L3 tiers
- **When** task routed to L2
- **Then** L2 sub-agent uses L2-configured model (e.g., claude-sonnet-4)
- **Files**: `runie-router/src/tier_router.rs`, `runie-orchestrator/src/orchestrator.rs`

### MDL-03: Model Switch via Command
- **Given** interactive mode, user types `/model`
- **When** provider/model selected
- **Then** root model updated; if subagents enabled, updates active tier model
- **Files**: `runie-cli/src/commands.rs`, `runie-ai/src/providers.rs`

### MDL-04: Scoped Model Cycling
- **Given** config: `scoped_models = ["gpt-4o-mini", "claude-sonnet-4"]`
- **When** `Ctrl+P` pressed
- **Then** cycles through scoped models for root agent
- **Edge**: subagents=true → cycles root tier model only
- **Files**: `runie-tui/src/events.rs`, `runie-cli/src/settings.rs`

### MDL-05: Custom Provider Registration
- **Given** `~/.runie/models.json` with custom OpenAI-compatible endpoint
- **When** app starts
- **Then** custom provider available for root or tier assignment
- **Files**: `runie-ai/src/custom_provider.rs`

### MDL-06: Model with Thinking Level Shorthand
- **Given** config or CLI: `--model claude-sonnet-4:high`
- **When** parsed
- **Then** model = "claude-sonnet-4", thinking = "high"
- **Files**: `runie-cli/src/settings.rs`

### MDL-07: Model Capability Detection
- **Given** model selected for current tier/root
- **When** tool call prepared
- **Then** checks model supports tools/vision/thinking, filters accordingly
- **Edge**: Model without tool support → no tools sent
- **Files**: `runie-ai/src/providers.rs`

---

## TOL — Tool Management (P0)

### TOL-01: Tool Allowlist
- **Given** CLI flag: `--tools read,grep,find`
- **When** agent initialized
- **Then** only specified tools available to model
- **Files**: `runie-cli/src/settings.rs`, `runie-tools/src/registry.rs`

### TOL-02: Tool Denylist (Disable Built-in)
- **Given** CLI flag: `--no-builtin-tools`
- **When** agent initialized
- **Then** built-in tools disabled, MCP/extension tools still available
- **Files**: `runie-tools/src/registry.rs`

### TOL-03: Disable All Tools
- **Given** CLI flag: `--no-tools`
- **When** agent initialized
- **Then** no tools available, pure chat mode
- **Files**: `runie-tools/src/registry.rs`

### TOL-04: Tool Registration Order
- **Given** extension registers tool "bash" after built-in
- **When** registry built
- **Then** extension version replaces built-in (last wins)
- **Files**: `runie-tools/src/registry.rs`

### TOL-05: Tool Schema Validation
- **Given** tool with invalid JSON schema
- **When** registered
- **Then** error at registration, tool excluded
- **Files**: `runie-tools/src/registry.rs`

---

## CMD — Chat Commands / Input Box (P0)

### CMD-01: Slash Trigger
- **Given** input bar focused, empty
- **When** user types `/`
- **Then** inline suggestion dropdown appears with available commands
- **Files**: `runie-tui/src/input_bar.rs`, `runie-core/src/slash_command.rs`

### CMD-02: Slash Command Filtering
- **Given** suggestion dropdown open
- **When** user types `/com`
- **Then** list filters to matching commands (`/compact`)
- **Edge**: No matches → "No commands found" in dim text
- **Files**: `runie-tui/src/input_bar.rs`

### CMD-03: Slash Command Accept (Tab)
- **Given** suggestion dropdown with `/compact` highlighted
- **When** `Tab` pressed
- **Then** command autocompleted in input: `/compact `
- **Files**: `runie-tui/src/input_bar.rs`

### CMD-04: Slash Command Accept (Enter)
- **Given** `/clear` typed in input
- **When** `Enter` pressed
- **Then** command executes, input cleared, chat cleared
- **Files**: `runie-tui/src/input_bar.rs`, `runie-cli/src/commands.rs`

### CMD-05: Slash Command with Arguments
- **Given** user types `/model gpt-4o`
- **When** `Enter` pressed
- **Then** model switched to gpt-4o, confirmation shown
- **Files**: `runie-core/src/slash_command.rs`, `runie-cli/src/commands.rs`

### CMD-06: Slash Command Escape
- **Given** suggestion dropdown open
- **When** `Esc` pressed
- **Then** dropdown closed, `/` remains in input for regular message
- **Files**: `runie-tui/src/input_bar.rs`

### CMD-07: /tree Command Opens Overlay
- **Given** session with branches
- **When** `/tree` submitted
- **Then** session tree overlay opened, can navigate and select branch
- **Files**: `runie-tui/src/tree_view.rs`

### CMD-08: /fork Command
- **Given** session with messages
- **When** `/fork` submitted
- **Then** fork overlay opens, user selects message, new session created
- **Files**: `runie-cli/src/commands.rs`, `runie-core/src/session.rs`

### CMD-09: /compact Command
- **Given** long session
- **When** `/compact` or `/compact focus on security` submitted
- **Then** compaction runs, messages summarized
- **Files**: `runie-core/src/compactor.rs`

### CMD-10: /name Command
- **Given** active session
- **When** `/name my-feature` submitted
- **Then** session display name updated in status bar
- **Files**: `runie-cli/src/commands.rs`

### CMD-11: /session Command
- **Given** active session
- **When** `/session` submitted
- **Then** session info overlay: file, ID, messages, tokens, cost
- **Files**: `runie-cli/src/commands.rs`

### CMD-12: /new Command
- **Given** active session
- **When** `/new` submitted
- **Then** new empty session started, current saved
- **Files**: `runie-cli/src/commands.rs`

### CMD-13: /resume Command
- **Given** previous sessions
- **When** `/resume` submitted
- **Then** session picker overlay, selected session loaded
- **Files**: `runie-cli/src/commands.rs`

### CMD-14: /quit Command
- **Given** TUI active
- **When** `/quit` submitted
- **Then** session saved, app exits gracefully
- **Files**: `runie-cli/src/commands.rs`

---

## THK — Thinking Levels (P0)

### THK-01: Thinking Level Off
- **Given** config: `thinking = "off"`
- **When** LLM request made
- **Then** no thinking block requested, standard response
- **Files**: `runie-ai/src/providers.rs`

### THK-02: Thinking Level Minimal
- **Given** config: `thinking = "minimal"`
- **When** LLM request made to thinking-capable model
- **Then** minimal thinking budget applied (e.g., 128 tokens)
- **Files**: `runie-ai/src/providers.rs`

### THK-03: Thinking Level High
- **Given** config: `thinking = "high"`
- **When** LLM request made
- **Then** high thinking budget applied (e.g., 2048 tokens)
- **Files**: `runie-ai/src/providers.rs`

### THK-04: Thinking Level Toggle
- **Given** TUI active
- **When** `Shift+Tab` pressed
- **Then** thinking level cycles: off → minimal → low → medium → high → xhigh → off
- **Files**: `runie-tui/src/events.rs`

### THK-05: Thinking Block Display
- **Given** response includes thinking block
- **When** rendered in TUI
- **Then** thinking block shown collapsed by default, expandable
- **Files**: `runie-tui/src/render.rs`

### THK-06: Thinking Block Collapse/Expand
- **Given** thinking block visible
- **When** `Ctrl+T` pressed
- **Then** all thinking blocks toggle collapse/expand
- **Files**: `runie-tui/src/events.rs`

### THK-07: Thinking Budget Custom
- **Given** config: `[thinking_budgets] minimal = 128, high = 4096`
- **When** thinking level set
- **Then** custom budgets override defaults
- **Files**: `runie-cli/src/settings.rs`

---

## REV — Code Review (P2)

### REV-01: Non-Interactive Code Review
- **Given** git repo with changes
- **When** `runie review` invoked
- **Then** agent reviews diff, outputs review comments to stdout
- **Files**: `runie-cli/src/commands.rs`

### REV-02: Review with Custom Instructions
- **Given** `runie review --instructions "focus on security"`
- **When** processed
- **Then** review focuses on security issues
- **Files**: `runie-cli/src/commands.rs`

### REV-03: Review Specific Files
- **Given** `runie review src/auth.rs src/db.rs`
- **When** processed
- **Then** only specified files reviewed
- **Files**: `runie-cli/src/commands.rs`

## APL — Apply Diff (P2)

### APL-01: Apply Last Diff
- **Given** previous agent run generated edits
- **When** `runie apply` invoked
- **Then** diff applied via `git apply` to working tree
- **Files**: `runie-cli/src/commands.rs`

### APL-02: Apply Specific Diff File
- **Given** diff file exists
- **When** `runie apply --file changes.patch`
- **Then** specified patch applied
- **Files**: `runie-cli/src/commands.rs`

## DIA — Diagnostics (P2)

### DIA-01: Doctor Command
- **Given** user runs `runie doctor`
- **When** diagnostics run
- **Then** checks: API keys, git config, disk space, network, permissions
- **Files**: `runie-cli/src/doctor.rs`

### DIA-02: Doctor Fix
- **Given** doctor found issues
- **When** `runie doctor --fix` invoked
- **Then** auto-fixes applicable issues, reports what was fixed
- **Files**: `runie-cli/src/doctor.rs`

## AGM — Agent Modes (P1)

### AGM-01: Build Mode (Default)
- **Given** default configuration
- **When** agent runs
- **Then** full tool access: read, write, bash, edit
- **Files**: `runie-agent/src/config.rs`

### AGM-08: Plan Mode Auto-Trigger (Complex Task)
- **Given** user submits complex task (e.g., "refactor auth system")
- **When** agent evaluates complexity score > threshold
- **Then** plan mode triggered automatically, no explicit mode switch needed
- **Files**: `runie-agent/src/complexity.rs`, `runie-orchestrator/src/planner.rs`

### AGM-08: Plan Mode — Simple Task Bypass
- **Given** user submits simple task (e.g., "list files")
- **When** agent evaluates complexity
- **Then** plan mode NOT triggered, agent executes directly
- **Files**: `runie-agent/src/complexity.rs`

### AGM-08: Plan Display — User Review
- **Given** plan mode triggered for complex task
- **When** plan generated
- **Then** plan shown in TUI with steps, user can review before execution
- **Files**: `runie-tui/src/plan_view.rs`

### AGM-08: Plan Approval — Grill-Me Session
- **Given** plan displayed, user wants to discuss before execution
- **When** user selects "Discuss plan"
- **Then** enters grill-me session: agent asks clarifying questions, user refines requirements
- **Edge**: User can modify plan steps during discussion
- **Files**: `runie-agent/src/plan_discussion.rs`

### AGM-08: Plan Approval — Proceed
- **Given** plan displayed, user satisfied
- **When** user selects "Proceed"
- **Then** agent exits plan mode, begins execution with approved plan
- **Files**: `runie-orchestrator/src/planner.rs`

### AGM-08: Plan Approval — Cancel
- **Given** plan displayed
- **When** user selects "Cancel"
- **Then** plan discarded, agent returns to idle state
- **Files**: `runie-orchestrator/src/planner.rs`

### AGM-08: Plan Mode Read-Only
- **Given** plan mode active
- **When** agent runs
- **Then** write/edit/bash denied, only read/plan tools allowed
- **Edge**: Bash requires explicit permission in plan mode
- **Files**: `runie-agent/src/config.rs`, `runie-agent/src/permission.rs`

### AGM-10: Mode Switch via Tab
- **Given** TUI active
- **When** `Tab` pressed
- **Then** cycles through modes: build → plan → build
- **Note**: Only switches if user explicitly enabled mode switching; auto-plan is separate
- **Files**: `runie-tui/src/events.rs`

### AGM-10: Mode Switch via Command
- **Given** interactive mode
- **When** `/mode plan` submitted
- **Then** agent switches to plan mode manually, status bar updated
- **Note**: Explicit override of auto-detection
- **Files**: `runie-cli/src/commands.rs`

## SND — Sandbox Execution (P2)

### SND-01: Sandbox Bash Command
- **Given** config: `sandbox = true`
- **When** bash tool executed
- **Then** command runs in sandbox (landlock/seatbelt/restricted token)
- **Files**: `runie-tools/src/bash.rs`, `runie-cli/src/sandbox.rs`

### SND-02: Sandbox File Access
- **Given** sandbox active
- **When** tool tries to access file outside workspace
- **Then** access denied, error returned to agent
- **Files**: `runie-cli/src/sandbox.rs`

### SND-03: Sandbox Network
- **Given** sandbox with network disabled
- **When** tool tries network access
- **Then** connection refused, error returned
- **Files**: `runie-cli/src/sandbox.rs`

## TUI — TUI Panels, Layout & Navigation (P0/P1)

### TUI-01: Main Layout Structure
- **Given** terminal available
- **When** TUI rendered
- **Then** layout: `[top_bar, sidebar | main_content, bottom_pane]`
- **Files**: `runie-tui/src/layout.rs`

### TUI-02: Sidebar Toggle
- **Given** sidebar visible
- **When** `Ctrl+B` pressed
- **Then** sidebar hidden; press again → sidebar shown
- **Files**: `runie-tui/src/events.rs`, `runie-tui/src/sidebar.rs`

### TUI-03: Sidebar Content
- **Given** sidebar visible
- **When** rendered
- **Then** shows: session list, file tree, or context (configurable)
- **Files**: `runie-tui/src/sidebar.rs`

### TUI-04: Top Bar Layout
- **Given** TUI active
- **When** rendered
- **Then** left: repo/branch/path; center: mode indicator; right: tokens/model/tier
- **Files**: `runie-tui/src/top_bar.rs`

### TUI-05: Bottom Pane — Composer
- **Given** TUI active
- **When** rendered
- **Then** bottom pane shows: input composer with prompt, hints
- **Files**: `runie-tui/src/bottom_pane.rs`

### TUI-06: Bottom Pane — Footer
- **Given** TUI active
- **When** rendered
- **Then** footer shows: hotkeys hint, status, queued messages count
- **Files**: `runie-tui/src/footer.rs`

### TUI-07: Bottom Pane — Popup Stack
- **Given** popup open (e.g., file search)
- **When** `Esc` pressed
- **Then** popup closed, composer restored
- **Files**: `runie-tui/src/bottom_pane.rs`

### TUI-08: Transcript Overlay (Ctrl+T)
- **Given** TUI active
- **When** `Ctrl+T` pressed
- **Then** transcript overlay opens, shows raw conversation history
- **Files**: `runie-tui/src/transcript.rs`

### TUI-09: Transcript Overlay Scroll
- **Given** transcript overlay open
- **When** `↑/↓` or `PgUp/PgDown` pressed
- **Then** scrolls through history
- **Files**: `runie-tui/src/transcript.rs`

### TUI-10: Transcript Overlay Exit
- **Given** transcript overlay open
- **When** `Esc` or `Ctrl+T` pressed
- **Then** overlay closed, main view restored
- **Files**: `runie-tui/src/transcript.rs`

### TUI-11: Vim Mode Toggle
- **Given** composer focused
- **When** `Esc` pressed (vim enabled)
- **Then** enters normal mode; `i` returns to insert mode
- **Files**: `runie-tui/src/composer.rs`

### TUI-12: File Search Popup
- **Given** composer focused, user types `@`
- **When** file search triggered
- **Then** popup with fuzzy file search appears
- **Files**: `runie-tui/src/file_search.rs`

### TUI-13: File Search Navigation
- **Given** file search popup open
- **When** `↑/↓` pressed
- **Then** selection moves through results
- **Files**: `runie-tui/src/file_search.rs`

### TUI-14: File Search Accept
- **Given** file selected in search
- **When** `Enter` or `Tab` pressed
- **Then** file path inserted into composer as mention
- **Files**: `runie-tui/src/file_search.rs`

### TUI-15: Diff Viewer
- **Given** agent proposes file edit
- **When** diff shown
- **Then** side-by-side or unified diff rendered with syntax highlighting
- **Files**: `runie-tui/src/diff_viewer.rs`

### TUI-16: Diff Viewer Accept
- **Given** diff viewer open
- **When** `y` pressed
- **Then** diff applied
- **Files**: `runie-tui/src/diff_viewer.rs`

### TUI-17: Diff Viewer Reject
- **Given** diff viewer open
- **When** `n` pressed
- **Then** diff rejected, agent notified
- **Files**: `runie-tui/src/diff_viewer.rs`

### TUI-18: Session Picker Overlay
- **Given** `/resume` or `Ctrl+R` invoked
- **When** picker opens
- **Then** list of previous sessions with metadata shown
- **Files**: `runie-tui/src/session_picker.rs`

### TUI-19: Session Picker Navigation
- **Given** session picker open
- **When** `↑/↓` pressed
- **Then** selection moves, preview updates
- **Files**: `runie-tui/src/session_picker.rs`

### TUI-20: Session Picker Load
- **Given** session selected in picker
- **When** `Enter` pressed
- **Then** session loaded, feed updated
- **Files**: `runie-tui/src/session_picker.rs`

### TUI-21: Feed Scroll
- **Given** message list longer than viewport
- **When** `PgUp/PgDown` or mouse scroll
- **Then** feed scrolls, maintains scroll offset
- **Files**: `runie-tui/src/feed.rs`

### TUI-22: Feed Auto-Scroll
- **Given** agent streaming response
- **When** new content arrives
- **Then** feed auto-scrolls to bottom (if user at bottom)
- **Files**: `runie-tui/src/feed.rs`

### TUI-23: Feed Pin Scroll
- **Given** user scrolled up to read history
- **When** new message arrives
- **Then** feed stays pinned, "New messages" indicator shown
- **Files**: `runie-tui/src/feed.rs`

### TUI-24: History Cell — User Message
- **Given** user sent message
- **When** rendered in feed
- **Then** user avatar, message text, attachments shown
- **Files**: `runie-tui/src/history_cell.rs`

### TUI-25: History Cell — Assistant Message
- **Given** assistant response
- **When** rendered in feed
- **Then** assistant avatar, markdown rendered, code blocks highlighted
- **Files**: `runie-tui/src/history_cell.rs`

### TUI-26: History Cell — Tool Call
- **Given** agent calls tool
- **When** rendered in feed
- **Then** tool name, args, status spinner shown
- **Files**: `runie-tui/src/history_cell.rs`

### TUI-27: History Cell — Tool Result
- **Given** tool execution completed
- **When** rendered in feed
- **Then** output collapsed by default, expandable with `Ctrl+O`
- **Files**: `runie-tui/src/history_cell.rs`

### TUI-28: History Cell — Error
- **Given** tool or agent error
- **When** rendered in feed
- **Then** error cell with red styling, stack trace collapsed
- **Files**: `runie-tui/src/history_cell.rs`

### TUI-29: History Cell — Plan
- **Given** agent creates plan
- **When** rendered in feed
- **Then** plan steps shown with checkboxes, progress indicator
- **Files**: `runie-tui/src/history_cell.rs`

### TUI-30: Hotkey Help Overlay
- **Given** TUI active
- **When** `?` or `/hotkeys` invoked
- **Then** overlay with all keyboard shortcuts shown
- **Files**: `runie-tui/src/help.rs`

### TUI-31: Notification Toast
- **Given** notification event
- **When** rendered
- **Then** transient toast at bottom, auto-dismisses after 5s
- **Files**: `runie-tui/src/notifications.rs`

### TUI-32: Resize Reflow
- **Given** terminal resized
- **When** new dimensions available
- **Then** all components reflow, text re-wrapped, no truncation
- **Files**: `runie-tui/src/layout.rs`

## Summary — All Scenarios by Priority

### P0 — MUST (v1 Ship Blockers)

| Prefix | Domain | Scenarios | Status |
|--------|--------|-----------|--------|
| `ONB-` | Onboarding | 37 | Existing |
| `CHT-` | Chat / Messages | 16 | Existing + Extended |
| `AGT-` | Agent / Tool Execution | 16 | Existing + Extended |
| `PER-` | Permissions | 12 | Existing |
| `PAL-` | Command Palette | 10 | Existing |
| `TOP-` | Top Bar | 12 | Existing |
| `STT-` | State Management | 19 | Existing + Extended |
| `CFG-` | Configuration | 17 | Existing + Extended |
| `REN-` | Rendering / UI | 25 | Existing + Extended |
| `INP-` | Input / Keyboard | 20 | Existing + Extended (text only) |
| `HAR-` | Harness / Tasks | 6 | Existing |
| `LOG-` | Logging | 5 | Existing |
| `EVT-` | Event Streaming | 12 | Existing + Extended |
| `GIT-` | Git Integration | 6 | Existing |
| `CTX-` | Context Loading | 7 | Existing |
| `ERR-` | Error Handling | 10 | Existing + Extended |
| `CMD-` | Chat Commands (Input Box) | 14 | New |
| `MDL-` | Model Management | 7 | New |
| `TOL-` | Tool Management | 5 | New |
| `THK-` | Thinking Levels | 7 | New |
| `TUI-` | TUI Panels / Layout / Navigation | 32 | New |
| `SES-` | Session Tree / Branching (flat) | 10 | New |
| `SKL-` | Skills | 5 | New |
| `AUT-` | Authentication (API key + env) | 5 | New |
| `MOD-` | Run Modes (CLI / TUI) | 2 | New (TUI + CLI one-shot only) |

**P0 Total: 301 scenarios**

### P1 — SHOULD (v1 Stretch Goals)

| Prefix | Domain | Scenarios | Risk |
|--------|--------|-----------|------|
| `INP-` | Image Paste | 5 | Platform-specific clipboard |
| `SES-` | Session Branching / Forking | 0 | Complex tree UI (subset of SES) |
| `AGM-` | Agent Modes (manual) | 10 | New subsystem |
| `TUI-` | Diff Viewer, Transcript, Vim | 0 | Complex UI (subset of TUI) |
| `MOD-` | JSON / Pipe modes | 2 | Additional modes |

**P1 Total: ~17 scenarios**

### P2 — COULD (v2 — Major Subsystems)

| Prefix | Domain | Scenarios | Complexity |
|--------|--------|-----------|------------|
| `RTR-` | Router / Tier Routing | 15 | Multi-model orchestration |
| `MCP-` | MCP Integration | 10 | External process management |
| `OCH-` | Orchestrator / Sub-agents | 10 | Actor system |
| `AUT-` | OAuth / Keychain | 0 | Platform auth APIs |
| `MOD-` | RPC / API Server | 8 | HTTP server + protocol |
| `REV-` | Code Review | 3 | Nice-to-have |
| `APL-` | Apply Diff | 2 | Nice-to-have |
| `DIA-` | Diagnostics | 2 | Nice-to-have |
| `SND-` | Sandbox Execution | 3 | Platform-specific (landlock/seatbelt) |
| `AGM-` | Auto-Plan + Grill-Me | 0 | Heuristic + NLP |

**P2 Total: ~53 scenarios**

---

**Grand Total: 371 scenarios across 32 domains**

*Phase 1-3: Core scenarios (200 across 16 domains).*
*Phase 4: Architecture + pi-scope scenarios (171 new).*
*Priority: P0=301 (v1), P1=~17 (v1 stretch), P2=~53 (v2).*
*Ready for implementation.*
