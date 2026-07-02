# Magic Numbers & Hardcoded Values — Audit Findings

## Provider / HTTP / retry

| File | Lines | Literal | Issue |
|------|-------|---------|-------|
| `crates/runie-provider/src/openai/mod.rs` | 32–33 | `120`, `10` | Request/connect timeout duplicated |
| `crates/runie-provider/src/model_client.rs` | 40–41 | `120`, `10` | Same timeouts duplicated |
| `crates/runie-core/src/actors/provider/factory.rs` | 32–33 | `120`, `10` | Same timeouts duplicated |
| `crates/runie-core/src/provider/provider_trait.rs` | 15–19 | `5`, `100`, `30`, `2.0` | Retry config exists but is unused by `with_retry` |
| `crates/runie-core/src/provider/provider_trait.rs` | 123,126,130 | `401`, `403`, `429`, `>=500` | Status classification duplicated in retry.rs |
| `crates/runie-provider/src/retry.rs` | 34,36,38 | `401`, `403`, `429`, `>=500` | Duplicate classification |
| `crates/runie-provider/src/openai/stream.rs` | 250–253 | `0`, `0` | Placeholder context-limit values |

## Actors / channels / timeouts

| File | Lines | Literal | Issue |
|------|-------|---------|-------|
| `crates/runie-core/src/actors/leader/actor.rs` | 122 | `32` | Leader command channel capacity |
| `crates/runie-core/src/actors/leader/actor.rs` | 100 | `1000` | Core event-bus capacity |
| `crates/runie-core/src/actors/leader/handle.rs` | 151 | `5` | Shutdown await timeout |
| `crates/runie-core/src/actors/config/handlers.rs` | 397 | `300` | Config watcher debounce ms |
| `crates/runie-core/src/actors/turn/speed_window.rs` | 17 | `1000` | Speed-window default capacity |
| `crates/runie-core/src/bus.rs` | 47 | `2` | Broadcast capacity multiplier |
| `crates/runie-core/src/actors/fff_indexer/ractor_fff_indexer.rs` | 183 | `10` | Max matches per file for grep |

## TUI / rendering

| File | Lines | Literal | Issue |
|------|-------|---------|-------|
| `crates/runie-tui/src/ui.rs` | 42–46 | `20`, `10` | Terminal-size margin threshold |
| `crates/runie-tui/src/ui.rs` | 69 | `.min(10)` | Max input box height |
| `crates/runie-tui/src/ui.rs` | 97–101 | `% 6`, `5 - raw_idx` | Spinner frame math duplicated with status_bar |
| `crates/runie-tui/src/status_bar.rs` | 67 | braille array | Spinner frame set duplicated |
| `crates/runie-tui/src/status_bar.rs` | 152–157 | `25`, `50`, `75`, `100` | Context-usage buckets |
| `crates/runie-tui/src/status_bar.rs` | 192–249 | `1_000`, `1_000_000` | Kilo/mega thresholds |
| `crates/runie-tui/src/popups.rs` | 48,64 | `.min(8)`, `.take(8)` | Max path suggestions |
| `crates/runie-tui/src/popups.rs` | 51–53 | `+1`, `- (4 + max_height)`, `.max(20)` | Popup position/size math |
| `crates/runie-tui/src/popups.rs` | 92–93 | `60`, `18`, `20`, `6`, `4` | Palette popup clamps/margins |
| `crates/runie-tui/src/popups/panel/form.rs` | many | `saturating_sub(3)`, `saturating_sub(6).max(12)`, etc. | Form layout math |
| `crates/runie-tui/src/popups/panel/list.rs` | 118,151,157,221,244,273,283 | separators, indents, widths | Popup list constants |
| `crates/runie-tui/src/message/support.rs` | 45,52,66,69,77,95,178,183 | glyphs and wrap widths | Blockquote/tool status formatting |
| `crates/runie-tui/src/pace.rs` | 44–48 | `/20.0`, `clamp(2,24)`, `+10` | Adaptive pacing constants |
| `crates/runie-tui/src/ui_actor.rs` | 199 | `16` | Effect forwarder channel capacity |
| `crates/runie-tui/src/popups/permission.rs` | 54,124 | `100`, `50` | Content/JSON preview limits |
| `crates/runie-tui/src/diff.rs` | 85 | `4` | Diff gutter width |
| `crates/runie-core/src/layout.rs` | 85,112,197 | `saturating_sub(2)` | Message width margin duplicated with TUI |
| `crates/runie-core/src/dialog/builders/session.rs` | 24–25 | `50` | Session-tree label truncation |

## Tools / commands

| File | Lines | Literal | Issue |
|------|-------|---------|-------|
| `crates/runie-agent/src/tool/grep.rs` | 53 | `100` | Default max grep matches |
| `crates/runie-agent/src/tool/find.rs` | 39 | `100` | Default max find results |
| `crates/runie-agent/src/tool/find.rs` | 109,115 | `10` | Find fallback max depth |
| `crates/runie-agent/src/tool/find_definitions.rs` | 206 | `5` | Max matches per file |
| `crates/runie-agent/src/tool/search/modes.rs` | 62 | `200`, `1` | Snippet truncation limits |
| `crates/runie-core/src/tool/format.rs` | 122,129,140,144 | `1000`, `1_000_000`, `1_000_000_000` | Byte-formatting thresholds |
| `crates/runie-core/src/tool/format.rs` | 160 | `60.0` | Sub-minute duration threshold |
| `crates/runie-core/src/tool/shim/mod.rs` | 255 | `"call_0"` | Fallback tool-call id duplicated |
| `crates/runie-core/src/tool/parse/mod.rs` | 48 | `"call_0"` | Same fallback id |
| `crates/runie-core/src/tool/shim/mod.rs` | 71 | `3` | Legacy tool colon split count |
| `crates/runie-core/src/tool/shim/mod.rs` | 42 | `5` | Offset into `"TOOL:"` string |
| `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` | 50,57 | `"2000"`, `"0"` | Form default placeholders |
| `crates/runie-core/src/commands/dsl/handlers/session/run.rs` | 37–38,65 | `0`, `&"2000"` | Fork fallback index / compact default |

## Config / state / UI strings

| File | Lines | Literal | Issue |
|------|-------|---------|-------|
| `crates/runie-core/src/config/config_impl.rs` | 494 | `"runie"` | Default theme name duplicated |
| `crates/runie-core/src/model/state/session.rs` | 150 | `"runie"` | Same default theme name |
| `crates/runie-core/src/config/config_impl.rs` | 438 | `"mock"`, `"echo"` | Mock provider/model defaults |
| `crates/runie-core/src/model/state/view.rs` | 117,118 | `20`, `80` | Default viewport height/width |
| `crates/runie-core/src/model/state/input.rs` | 49 | placeholder string | Hardcoded UI copy |
| `crates/runie-core/src/model/state/types.rs` | 77–103 | prompt suffixes / I Ching symbols | Hardcoded copy/symbols |
| `crates/runie-core/src/update/system.rs` | 25 | `Duration::from_secs(5)` | Transient message timeout |
| Many handler files | many | usage/error/copy strings | Scattered UI copy |
