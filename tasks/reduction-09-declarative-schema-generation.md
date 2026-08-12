# Reduction 09: declarative schema generation

Status: adopted

Identify repetitive registries, dispatch tables, and metadata suitable for
typed declarations or narrowly scoped macros.

Acceptance: generated output is type checked, inspectable, and does not hide
async lifecycle or domain decisions.

Progress: `component_specs!` and the existing view/layout DSLs now generate
repetitive immutable view metadata. Domain reducers and async lifecycle remain
ordinary Rust.

The paint layer also uses `paint!` for repetitive immutable paint rows.
Static provider and theme registries now use typed constant tables instead of
large function bodies; `declare_reducer_actor!` generates the remaining
typed actor handle boilerplate.
Palette slash-command and description metadata now use declarative Rust
macros, keeping the `PaletteAction` methods as thin typed accessors.
Model-declared effort levels now use one typed `ThinkingLevel::ALL` vocabulary
and `ThinkingLevelMap::declared` projection for picker options, selection
validation, YAML replay, and model capability checks.
Tool-card paint intent conversion is now one typed mapping shared by the
renderer and pure paint projection, removing a duplicate dispatch table.
Feed navigation-to-snapshot projection now uses one typed field declaration
macro, including explicit clone fields and the derived selected-member field,
so the projection schema is data-shaped instead of repeated assignments.
Activity-tool aliases now use a typed grouped alias macro, keeping the
classifier vocabulary declarative and exhaustively mapped to `ActivityKind`.
Builtin theme names now use one typed macro table that generates both the
loader dispatch and its test inventory, eliminating the split hand-written
match while keeping every theme mapping inspectable as data.
Unused legacy palette metadata macros were removed after confirming no call
sites, leaving one live declaration path for command names and descriptions.
The live slash command and description metadata now come from one typed table;
the macro generates both exhaustive accessors from each action row.
Normalized feed-fact reset groups now also come from one typed field table,
keeping activity/workflow ownership explicit while removing repeated reset
assignments.
`LineKind` variants and their transcript prefix glyphs now share one
macro-backed table, while semantic classification predicates remain explicit.
Palette section membership now uses one macro-backed classification table,
removing the split helper predicates while retaining an explicit fallback.
Plugin capability kinds and their package directories now share one
macro-backed declaration, keeping plugin entrypoint routing data-shaped.
Session journal record types now use one macro-backed wire-name table,
including the typed/generic operation aliases.
Session-lane operation variants, typed kinds, and wire names now share one
macro-backed table for decode, reverse projection, and parsing.
Provider effort wire fields now use one macro-backed key table, keeping
adapter-specific spellings explicit without duplicating the enum mapping.
Semantic theme tokens and their stable Opaline names now share one typed
macro-backed table, keeping renderer vocabulary data-shaped.
Stop-reason telemetry and display labels now share one typed table while
retaining their intentional `tool_use` versus `toolUse` representations.
Operation-record kinds now also generate both wire-name encoding and
compatibility decoding from one typed table.
Web-search provider snippet fields now use one typed wire-format table for
Generic, Brave, and Tavily normalization.
Dialog kinds now generate their renderer hint strings from one model-owned
table, removing the widget-local duplicate; the same enum is serde-backed for
fixture replay.
Todo plan summaries now own their terminal projection, so `/plan` and
`/view-plan` consume the same data-shaped formatter instead of rebuilding the
status row in the TUI route.
Bounded workspace and background tool outputs now share the core
`OutputFacts` projection for bytes, lines, and truncation, while their public
domain rows retain their stable shapes. Their bounded Unicode-safe preview is
also one shared output projection, so tool cards and background jobs cannot
drift in preview semantics.
Executor built-in tool routing now uses one explicit macro-backed name/handler
table, keeping the closed vocabulary data-shaped while leaving each async
handler and lifecycle boundary inspectable.
Background job statuses now use one macro-backed wire-name table for both
domain serialization and `/jobs` query validation, eliminating a duplicate
command-local string vocabulary.
MCP stdio and HTTP lifecycle statuses now use the same macro-generated forward
and reverse wire vocabulary, and `/mcps` query validation consumes it directly.
MCP transport filters now use a typed macro-generated `stdio`/`http` wire
vocabulary instead of a command-local string list, and the transport enum is
serde-replayable data with round-trip coverage.
Scheduler terminal metric fields now use one macro-backed declaration for
stable names and row projection, while event reduction remains explicit.
Provider request profiles now also declare whether effort is nested, so
request shaping consumes the profile data instead of a provider-specific
conditional.
Fixed palette-to-extended-command routes now also come from one typed macro
table, eliminating repeated command construction while preserving the same
actor-owned route boundary.

Completion evidence: the workspace lint guard, full workspace tests, replay
tests, and live TUI smoke all pass; repetitive registries and projections use
typed declarations while reducers and asynchronous lifecycle decisions remain
inspectable ordinary Rust as required by the acceptance criteria.
