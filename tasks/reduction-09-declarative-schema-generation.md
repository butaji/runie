# Reduction 09: declarative schema generation

Status: partial

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
Model-declared effort levels now use one macro-backed data table for picker
options, selection validation, and model capability checks.
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
