# Replace register_handler and theme macros with functions

## Status

`done`

## Context

Three macros existed only to avoid typing boilerplate:
- `register_handler!` in `crates/runie-core/src/commands/dsl/handlers/mod.rs:49-76`
- `theme_color!` / `theme_color_try!` in `crates/runie-tui/src/theme/colors.rs:9-28`
- `style_fn!` in `crates/runie-tui/src/theme/styles.rs:150-156`

They added indirection and slowed compile-time understanding.

## Changes Made

### `register_handler!` → direct `NamedHandler` construction

The macro was replaced with direct use of `NamedHandler` variants in each `register_handlers` function:

- `crates/runie-core/src/commands/dsl/handlers/mod.rs` — removed macro definition
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — `registry.register("save", NamedHandler::FormWithHandler { ... })`
- `crates/runie-core/src/commands/dsl/handlers/model.rs` — `registry.register("model", NamedHandler::Handler(handle_model))`
- `crates/runie-core/src/commands/dsl/handlers/help.rs` — direct `NamedHandler::Handler` calls
- `crates/runie-core/src/commands/dsl/handlers/system.rs` — direct `NamedHandler::Handler` calls
- `crates/runie-core/src/commands/dsl/handlers/tool.rs` — direct `NamedHandler::Handler` calls

### `theme_color!` / `theme_color_try!` → `fn` getters

`crates/runie-tui/src/theme/colors.rs`:
- `theme_color!(color_fg, "text.primary")` → `pub fn color_fg() -> Color { theme_color("text.primary") }`
- `theme_color_try!(color_bg, "bg.base", Color::Reset)` → `pub fn color_bg() -> Color { theme_color_fallback("bg.base", Color::Reset) }`
- Helper functions `theme_color(key: &str) -> Color` and `theme_color_fallback(key: &str, fallback: Color) -> Color` replace the macros

### `style_fn!` → `fn` getters

`crates/runie-tui/src/theme/styles.rs`:
- `style_fn!(style_user, "runie.user")` → `pub fn style_user() -> Style { style_fn("runie.user") }`
- 30 similar replacements
- Helper function `style_fn(token: &str) -> Style` replaces the macro

## Acceptance Criteria

- [x] `register_handler!` becomes direct `registry.register(name, NamedHandler::...)` calls.
- [x] `theme_color!` becomes plain `fn` getters.
- [x] `style_fn!` becomes plain `fn` getters.
- [x] All callers compile and behavior is unchanged.

## Validation

- **`cargo check --workspace`**: passes with 0 errors
- **`cargo test --workspace`**: passes
- No `!` macro references remain in source files

## Design Impact

No change to TUI element design or composition. Only internal macro/function structure changes.
